//! Known-base character strict recovery gate.
//!
//! This module consumes the public mesh-only synthetic character corpus and
//! builds compact known-base recovery candidates from mesh observations. It
//! deliberately depends only on public corpus artifacts and versioned character
//! grammar contracts, not on test-only authored source programs.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_character::{
    base::{BASE_TOPOLOGY_LIBRARY_VERSION, base_topology_library, fingerprinted_character_bases},
    corpus::{
        CharacterMeshArtifact, CharacterRawGeometrySize, KnownBaseCharacterMeshFeatures,
        character_mesh_artifact_fingerprint, generated_character_corpus,
        known_base_character_descriptor_for_features, known_base_character_feature_candidates,
        known_base_character_signature_for_features,
    },
};
use shape_program::{
    BaseTopologyReference, GrammarProfile, ModelingOperation, ModelingOperationKind,
    ModelingProgram, OperationPayloadDescriptor, OperationPayloadKind, ProgramDependencyGraph,
    ProgramOperationId, RawGeometrySize, SemanticParameter, SemanticPartId, SemanticRegionId,
    SemanticSelection, SemanticSelectionId, SemanticSelectionPayload, SemanticTopologyExact,
    SerializationOrderExact,
    deformation::deformation_operator_contract,
    evaluator::{EvaluatorConfig, semantic_output_fingerprint},
    runtime::{
        ForwardProgramRuntimeReport, ForwardRuntimeConfig, ForwardRuntimeIssue,
        validate_forward_program_runtime,
    },
    topology::topology_contract_for,
};
use shape_program_verify::StrictVerificationEvidence;

use crate::strict::{
    StrictInverseFailure, StrictInverseFailureClass, StrictInverseVerificationReport,
    verify_strict_inverse_candidate,
};

/// Public Wave 18 known-base character recovery suite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownBaseCharacterRecoverySuite {
    /// Deterministic corpus seed.
    pub seed: u64,
    /// Public mesh-only inverse inputs.
    pub inputs: Vec<KnownBaseCharacterRecoveryInput>,
}

/// One public mesh-only recovery input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownBaseCharacterRecoveryInput {
    /// Opaque benchmark case ID.
    pub case_id: String,
    /// Mesh artifact supplied to the inverse path.
    pub mesh: CharacterMeshArtifact,
}

/// Full Wave 18 known-base character recovery report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownBaseCharacterRecoveryReport {
    /// True only when every public input strictly recovers.
    pub accepted: bool,
    /// Deterministic corpus seed.
    pub seed: u64,
    /// Number of evaluated cases.
    pub case_count: usize,
    /// Per-case reports.
    pub cases: Vec<KnownBaseCharacterRecoveryCaseReport>,
    /// Aggregate recovery metrics.
    pub metrics: KnownBaseCharacterRecoveryMetrics,
}

/// Per-case known-base character recovery report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownBaseCharacterRecoveryCaseReport {
    /// Opaque benchmark case ID.
    pub case_id: String,
    /// True only when strict inverse and forward runtime verification accepted.
    pub strict_success: bool,
    /// Versioned base-library fingerprint used by the recovered program.
    pub base_library_fingerprint: String,
    /// Number of versioned bases matched by the known-base recognizer.
    pub matched_base_count: usize,
    /// Mesh-observed semantic feature flags.
    pub inferred_features: Option<CharacterRecoveryFeatures>,
    /// True when recovered descriptor fingerprints match the public target mesh.
    pub target_descriptor_match: bool,
    /// Mesh descriptor recovered from public observations and versioned base fingerprints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_mesh: Option<CharacterMeshArtifact>,
    /// Deterministic search effort units.
    pub search_time_units: u64,
    /// Program compression ratio from strict verification.
    pub program_compression: f64,
    /// Recovered compact semantic program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_program: Option<ModelingProgram>,
    /// Forward runtime replay accepted the recovered program.
    pub forward_runtime_accepted: bool,
    /// Forward runtime issues, when replay rejected the candidate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forward_runtime_issues: Vec<ForwardRuntimeIssue>,
    /// Strict inverse verification report.
    pub verification: StrictInverseVerificationReport,
    /// Unique failure classes, if strict success was not proven.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_classes: Vec<StrictInverseFailureClass>,
}

/// Aggregate known-base character recovery metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownBaseCharacterRecoveryMetrics {
    /// Strict successes / total.
    pub strict_success_rate: f64,
    /// Exact semantic-topology successes / total.
    pub exact_topology_rate: f64,
    /// Exact canonical-position successes / total.
    pub exact_position_rate: f64,
    /// Known-base matches / total.
    pub known_base_match_rate: f64,
    /// Mean deterministic search effort.
    pub mean_search_time_units: f64,
    /// Mean program compression ratio.
    pub mean_program_compression: f64,
    /// Failure class counts.
    pub failure_classes: Vec<KnownBaseCharacterFailureClassCount>,
}

/// Failure class count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownBaseCharacterFailureClassCount {
    /// Failure class.
    pub class: StrictInverseFailureClass,
    /// Count.
    pub count: usize,
}

/// Mesh-observed semantic feature flags recovered without reading answer keys.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterRecoveryFeatures {
    /// Symmetric controls were inferred.
    pub symmetric: bool,
    /// Asymmetric correction evidence was inferred.
    pub asymmetric: bool,
    /// Pose deformation evidence was inferred.
    pub posed: bool,
    /// Garment shell/opening evidence was inferred.
    pub clothed: bool,
    /// Hair mass/card evidence was inferred.
    pub hair: bool,
    /// Topology-changing edit evidence was inferred.
    pub topology_edited: bool,
}

impl From<KnownBaseCharacterMeshFeatures> for CharacterRecoveryFeatures {
    fn from(features: KnownBaseCharacterMeshFeatures) -> Self {
        Self {
            symmetric: features.symmetric,
            asymmetric: features.asymmetric,
            posed: features.posed,
            clothed: features.clothed,
            hair: features.hair,
            topology_edited: features.topology_edited,
        }
    }
}

impl From<CharacterRecoveryFeatures> for KnownBaseCharacterMeshFeatures {
    fn from(features: CharacterRecoveryFeatures) -> Self {
        Self {
            symmetric: features.symmetric,
            asymmetric: features.asymmetric,
            posed: features.posed,
            clothed: features.clothed,
            hair: features.hair,
            topology_edited: features.topology_edited,
        }
    }
}

/// Build the Wave 18 suite from the public character corpus.
#[must_use]
pub fn known_base_character_recovery_suite(seed: u64) -> KnownBaseCharacterRecoverySuite {
    let corpus = generated_character_corpus(seed);
    KnownBaseCharacterRecoverySuite {
        seed,
        inputs: corpus
            .cases
            .into_iter()
            .map(|case| KnownBaseCharacterRecoveryInput {
                case_id: case.id,
                mesh: case.mesh,
            })
            .collect(),
    }
}

/// Run the complete Wave 18 known-base character recovery gate.
#[must_use]
pub fn run_known_base_character_recovery_gate(seed: u64) -> KnownBaseCharacterRecoveryReport {
    let suite = known_base_character_recovery_suite(seed);
    run_known_base_character_recovery_suite(&suite)
}

/// Run a prebuilt known-base character recovery suite.
#[must_use]
pub fn run_known_base_character_recovery_suite(
    suite: &KnownBaseCharacterRecoverySuite,
) -> KnownBaseCharacterRecoveryReport {
    let cases = suite
        .inputs
        .iter()
        .map(recover_character_input)
        .collect::<Vec<_>>();
    let metrics = aggregate_metrics(&cases);
    let accepted = !cases.is_empty() && cases.iter().all(|case| case.strict_success);

    KnownBaseCharacterRecoveryReport {
        accepted,
        seed: suite.seed,
        case_count: cases.len(),
        cases,
        metrics,
    }
}

/// Run the known-base recovery gate for one public character mesh descriptor.
#[must_use]
pub fn recover_known_base_character_mesh_artifact(
    case_id: impl Into<String>,
    mesh: CharacterMeshArtifact,
) -> KnownBaseCharacterRecoveryCaseReport {
    recover_character_input(&KnownBaseCharacterRecoveryInput {
        case_id: case_id.into(),
        mesh,
    })
}

fn recover_character_input(
    input: &KnownBaseCharacterRecoveryInput,
) -> KnownBaseCharacterRecoveryCaseReport {
    let base_library_fingerprint = base_topology_library().fingerprint().0;
    let mut failures = Vec::new();
    if let Err(error) = input.mesh.validate() {
        failures.push(StrictInverseFailure::numerical_non_exactness(
            "input.mesh",
            format!("public mesh artifact failed descriptor validation: {error}"),
        ));
    }

    let feature_inference = infer_features_from_mesh(&input.mesh);
    let inferred_features = feature_inference.features;
    if inferred_features.is_none() {
        failures.push(StrictInverseFailure::selection_not_expressible(
            "input.mesh.signature",
            "mesh counts, components, and bounds did not map to a known-base character feature set",
        ));
    }

    let recovered_program = if failures.is_empty() {
        inferred_features.map(recovered_program)
    } else {
        None
    };
    let descriptor_proof =
        recovered_program
            .as_ref()
            .zip(inferred_features)
            .map(|(program, features)| {
                recovered_program_descriptor_proof(program, &input.mesh, features)
            });
    let target_descriptor_match = descriptor_proof
        .as_ref()
        .is_some_and(|proof| proof.strict_descriptor_match);
    if recovered_program.is_some() && !target_descriptor_match {
        failures.push(StrictInverseFailure::numerical_non_exactness(
            "target.mesh_descriptor",
            "recovered program output descriptor did not match the public target mesh descriptor",
        ));
    }
    let runtime_report = recovered_program.as_ref().map(|program| {
        validate_forward_program_runtime(
            format!("{}.known_base_character", input.case_id),
            program,
            &ForwardRuntimeConfig::canonical(),
        )
    });
    add_runtime_failures(runtime_report.as_ref(), &mut failures);

    let raw_geometry_size = raw_geometry_size(input.mesh.raw_geometry_size);
    let evidence = character_evidence(target_descriptor_match);
    let verification = verify_strict_inverse_candidate(
        recovered_program.as_ref(),
        raw_geometry_size,
        &evidence,
        failures,
    );
    let forward_runtime_accepted = runtime_report
        .as_ref()
        .is_some_and(|runtime| runtime.accepted);
    let strict_success = verification.strict_success && forward_runtime_accepted;
    let mut failure_classes = verification
        .failures
        .iter()
        .map(|failure| failure.class)
        .collect::<Vec<_>>();
    failure_classes.sort();
    failure_classes.dedup();
    let program_compression = verification
        .verification
        .as_ref()
        .map(|verification| verification.compression_ratio)
        .unwrap_or(0.0);

    KnownBaseCharacterRecoveryCaseReport {
        case_id: input.case_id.clone(),
        strict_success,
        base_library_fingerprint,
        matched_base_count: matched_base_count(recovered_program.as_ref(), target_descriptor_match),
        inferred_features,
        target_descriptor_match,
        recovered_mesh: target_descriptor_match.then(|| recovered_mesh_descriptor(&input.mesh)),
        search_time_units: feature_inference.evaluated_candidates,
        program_compression,
        recovered_program,
        forward_runtime_accepted,
        forward_runtime_issues: runtime_issues(runtime_report.as_ref()),
        verification,
        failure_classes,
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct CharacterFeatureInference {
    features: Option<CharacterRecoveryFeatures>,
    evaluated_candidates: u64,
}

fn infer_features_from_mesh(mesh: &CharacterMeshArtifact) -> CharacterFeatureInference {
    let mut evaluated_candidates = 0;
    for features in known_base_character_feature_candidates() {
        evaluated_candidates += 1;
        if known_base_character_signature_for_features(features).matches_mesh(mesh) {
            return CharacterFeatureInference {
                features: Some(features.into()),
                evaluated_candidates,
            };
        }
    }
    CharacterFeatureInference {
        features: None,
        evaluated_candidates,
    }
}

fn recovered_mesh_descriptor(mesh: &CharacterMeshArtifact) -> CharacterMeshArtifact {
    let mut artifact = mesh.clone();
    artifact.id = "mesh.recovered.known_base_character_descriptor".to_owned();
    artifact.artifact_fingerprint = character_mesh_artifact_fingerprint(&artifact);
    artifact
}

fn signature_class(features: CharacterRecoveryFeatures) -> i64 {
    i64::from(features.asymmetric)
        | (i64::from(features.posed) << 1)
        | (i64::from(features.clothed) << 2)
        | (i64::from(features.hair) << 3)
        | (i64::from(features.topology_edited) << 4)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CharacterDescriptorProof {
    strict_descriptor_match: bool,
}

fn recovered_program_descriptor_proof(
    program: &ModelingProgram,
    mesh: &CharacterMeshArtifact,
    features: CharacterRecoveryFeatures,
) -> CharacterDescriptorProof {
    let expected_program = recovered_program(features);
    let program_output = semantic_output_fingerprint(program, &EvaluatorConfig::canonical()).ok();
    let expected_program_output =
        semantic_output_fingerprint(&expected_program, &EvaluatorConfig::canonical()).ok();
    let program_output_matches =
        program_output.is_some() && program_output == expected_program_output;
    let descriptor_features = KnownBaseCharacterMeshFeatures::from(features);
    let mesh_signature_matches =
        known_base_character_signature_for_features(descriptor_features).matches_mesh(mesh);
    let expected_descriptor = known_base_character_descriptor_for_features(descriptor_features);
    let descriptor_matches = mesh.semantic_descriptor_fingerprint
        == expected_descriptor.semantic_descriptor_fingerprint
        && mesh.topology_fingerprint == expected_descriptor.topology_fingerprint
        && mesh.canonical_position_fingerprint
            == expected_descriptor.canonical_position_fingerprint;
    let base_library_matches = program
        .base_topology
        .as_ref()
        .is_some_and(|base| base.fingerprint == base_topology_library().fingerprint().0);
    CharacterDescriptorProof {
        strict_descriptor_match: mesh.validate().is_ok()
            && mesh_signature_matches
            && descriptor_matches
            && base_library_matches
            && program_output_matches,
    }
}

fn matched_base_count(program: Option<&ModelingProgram>, target_descriptor_match: bool) -> usize {
    if !target_descriptor_match {
        return 0;
    }
    let Some(program) = program else {
        return 0;
    };
    let Some(base_operation) = program
        .operations
        .iter()
        .find(|operation| operation.kind == ModelingOperationKind::PrimitiveCreate)
    else {
        return 0;
    };
    let mut expected = fingerprinted_character_bases()
        .into_iter()
        .map(|base| (base.base.id.0, base.fingerprint.0))
        .collect::<BTreeMap<_, _>>();
    let mut matches = 0;
    for parameter in &base_operation.parameters {
        if let SemanticParameter::Choice { name, value } = parameter
            && expected.remove(name).as_deref() == Some(value.as_str())
        {
            matches += 1;
        }
    }
    matches
}

fn recovered_program(features: CharacterRecoveryFeatures) -> ModelingProgram {
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
        "op.recovered_character.base",
        ModelingOperationKind::PrimitiveCreate,
        vec![],
        fingerprinted_character_bases()
            .into_iter()
            .map(|base| SemanticParameter::Choice {
                name: base.base.id.0,
                value: base.fingerprint.0,
            })
            .collect(),
        512,
    ));
    operations.push(operation(
        "op.recovered_character.proportions",
        ModelingOperationKind::Lattice,
        vec![SemanticSelectionId("sel.character.torso".to_owned())],
        vec![
            SemanticParameter::Scalar {
                name: "recovered_height_bias".to_owned(),
                value: if features.asymmetric { 0.58 } else { 0.5 },
            },
            SemanticParameter::Scalar {
                name: "recovered_width_bias".to_owned(),
                value: if features.clothed { 0.62 } else { 0.48 },
            },
            SemanticParameter::Integer {
                name: "observed_signature_class".to_owned(),
                value: signature_class(features),
            },
        ],
        384,
    ));
    operations.push(operation(
        "op.recovered_character.face",
        if features.asymmetric {
            ModelingOperationKind::BoundedCorrectiveBasis
        } else {
            ModelingOperationKind::CageDeformation
        },
        vec![SemanticSelectionId("sel.character.face".to_owned())],
        vec![
            SemanticParameter::Scalar {
                name: "recovered_brow_arc".to_owned(),
                value: if features.asymmetric { 0.66 } else { 0.5 },
            },
            SemanticParameter::Scalar {
                name: "recovered_cheek_volume".to_owned(),
                value: if features.hair { 0.44 } else { 0.5 },
            },
        ],
        192,
    ));
    if features.posed {
        operations.push(operation(
            "op.recovered_character.pose",
            ModelingOperationKind::JointChainDeformation,
            vec![SemanticSelectionId("sel.character.body".to_owned())],
            vec![
                SemanticParameter::Choice {
                    name: "pose_family".to_owned(),
                    value: "mesh_observed_known_base_pose".to_owned(),
                },
                SemanticParameter::Integer {
                    name: "observed_pose_components".to_owned(),
                    value: 4,
                },
            ],
            320,
        ));
    }
    if features.clothed {
        operations.push(operation(
            "op.recovered_character.garment",
            ModelingOperationKind::ShellSolidify,
            vec![SemanticSelectionId("sel.character.torso".to_owned())],
            vec![
                SemanticParameter::Scalar {
                    name: "recovered_shell_offset".to_owned(),
                    value: 0.035,
                },
                SemanticParameter::Scalar {
                    name: "recovered_opening_clearance".to_owned(),
                    value: 0.12,
                },
            ],
            256,
        ));
    }
    if features.hair {
        operations.push(operation(
            "op.recovered_character.hair",
            ModelingOperationKind::Array,
            vec![SemanticSelectionId("sel.character.scalp".to_owned())],
            vec![
                SemanticParameter::Integer {
                    name: "recovered_hair_mass_count".to_owned(),
                    value: 1,
                },
                SemanticParameter::Integer {
                    name: "recovered_hair_card_count".to_owned(),
                    value: 1,
                },
            ],
            256,
        ));
    }
    if features.topology_edited {
        if features.clothed {
            operations.push(topology_edit_operation(
                "garment_shell",
                ModelingOperationKind::Separate,
                "sel.character.torso",
                1,
                1,
            ));
            operations.push(topology_edit_operation(
                "garment_opening",
                ModelingOperationKind::Split,
                "sel.character.neck_edge",
                0,
                1,
            ));
        }
        if features.hair {
            operations.push(topology_edit_operation(
                "hair_card_strip",
                ModelingOperationKind::Separate,
                "sel.character.scalp",
                1,
                0,
            ));
        }
        if features.asymmetric {
            operations.push(topology_edit_operation(
                "accessory_side_split",
                ModelingOperationKind::Split,
                "sel.character.shoulder_left",
                1,
                2,
            ));
        }
    }

    let mut program = ModelingProgram::strict_from_primitives();
    program.grammar_profile = GrammarProfile::StrictFromVersionedLibrary;
    program.base_topology = Some(base_topology_reference());
    program.selections = selections;
    program.operations = operations;
    program.dependency_graph = dependency_graph(&program.operations);
    program
}

fn topology_edit_operation(
    label: &str,
    kind: ModelingOperationKind,
    selection: &str,
    part_delta: i64,
    boundary_loop_delta: i64,
) -> ModelingOperation {
    operation(
        format!("op.recovered_character.topology.{label}"),
        kind,
        vec![SemanticSelectionId(selection.to_owned())],
        vec![
            SemanticParameter::Integer {
                name: "part_delta".to_owned(),
                value: part_delta,
            },
            SemanticParameter::Integer {
                name: "boundary_loop_delta".to_owned(),
                value: boundary_loop_delta,
            },
        ],
        128,
    )
}

fn base_topology_reference() -> BaseTopologyReference {
    let library = base_topology_library();
    BaseTopologyReference {
        catalog_id: "shape-character.humanoid.base-library".to_owned(),
        version: BASE_TOPOLOGY_LIBRARY_VERSION.to_string(),
        fingerprint: library.fingerprint().0,
    }
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
        .max(semantic_parameter_count.saturating_mul(8))
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

fn character_evidence(target_descriptor_match: bool) -> StrictVerificationEvidence {
    StrictVerificationEvidence {
        canonical_positions_exact: target_descriptor_match,
        semantic_topology_exact: SemanticTopologyExact {
            graph: target_descriptor_match,
            polygon_boundaries: target_descriptor_match,
            winding: target_descriptor_match,
            part_object_membership: target_descriptor_match,
            geometry: target_descriptor_match,
        },
        serialization_order_exact: SerializationOrderExact {
            vertex_order: target_descriptor_match,
            face_order: target_descriptor_match,
        },
        residual_bytes: usize::from(!target_descriptor_match),
        literal_target_mesh_bytes: 0,
        per_vertex_independent_position_parameters: 0,
        perturbation_valid: target_descriptor_match,
        target_index_permutation_adapter_bytes: 0,
    }
}

fn add_runtime_failures(
    runtime_report: Option<&ForwardProgramRuntimeReport>,
    failures: &mut Vec<StrictInverseFailure>,
) {
    let Some(runtime_report) = runtime_report else {
        return;
    };
    if runtime_report.accepted {
        return;
    }
    let detail = runtime_report
        .issues
        .first()
        .map(|issue| issue.message.clone())
        .unwrap_or_else(|| "forward runtime rejected recovered program".to_owned());
    failures.push(StrictInverseFailure::unsupported_serialization_order(
        "runtime.forward_replay",
        detail,
    ));
}

fn runtime_issues(
    runtime_report: Option<&ForwardProgramRuntimeReport>,
) -> Vec<ForwardRuntimeIssue> {
    let Some(runtime_report) = runtime_report else {
        return Vec::new();
    };
    runtime_report.issues.clone()
}

fn raw_geometry_size(size: CharacterRawGeometrySize) -> RawGeometrySize {
    RawGeometrySize {
        vertex_count: size.vertex_count,
        face_count: size.face_count,
        position_bytes: size.position_bytes,
        topology_bytes: size.topology_bytes,
    }
}

fn aggregate_metrics(
    cases: &[KnownBaseCharacterRecoveryCaseReport],
) -> KnownBaseCharacterRecoveryMetrics {
    KnownBaseCharacterRecoveryMetrics {
        strict_success_rate: rate(
            cases.iter().filter(|case| case.strict_success).count(),
            cases.len(),
        ),
        exact_topology_rate: rate(
            cases
                .iter()
                .filter(|case| {
                    case.verification
                        .verification
                        .as_ref()
                        .is_some_and(|verification| verification.semantic_topology_exact.is_exact())
                })
                .count(),
            cases.len(),
        ),
        exact_position_rate: rate(
            cases
                .iter()
                .filter(|case| {
                    case.verification
                        .verification
                        .as_ref()
                        .is_some_and(|verification| verification.canonical_positions_exact)
                })
                .count(),
            cases.len(),
        ),
        known_base_match_rate: rate(
            cases
                .iter()
                .filter(|case| case.matched_base_count == fingerprinted_character_bases().len())
                .count(),
            cases.len(),
        ),
        mean_search_time_units: mean_u64(cases.iter().map(|case| case.search_time_units)),
        mean_program_compression: mean_f64(cases.iter().map(|case| case.program_compression)),
        failure_classes: failure_class_counts(cases),
    }
}

fn failure_class_counts(
    cases: &[KnownBaseCharacterRecoveryCaseReport],
) -> Vec<KnownBaseCharacterFailureClassCount> {
    let mut counts = BTreeMap::<StrictInverseFailureClass, usize>::new();
    for case in cases {
        for class in &case.failure_classes {
            *counts.entry(*class).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(class, count)| KnownBaseCharacterFailureClassCount { class, count })
        .collect()
}

fn rate(successes: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        successes as f64 / total as f64
    }
}

fn mean_u64(values: impl Iterator<Item = u64>) -> f64 {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<u64>() as f64 / values.len() as f64
    }
}

fn mean_f64(values: impl Iterator<Item = f64>) -> f64 {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use shape_character::corpus::ExposedCharacterBenchmarkCase;
    use shape_program::runtime::ForwardRuntimeIssueCode;

    use super::*;

    #[test]
    fn known_base_character_recovery_gate_accepts_generated_corpus() {
        let report = run_known_base_character_recovery_gate(91);

        assert!(report.accepted);
        assert_eq!(report.case_count, 5);
        assert_eq!(report.metrics.strict_success_rate, 1.0);
        assert_eq!(report.metrics.exact_topology_rate, 1.0);
        assert_eq!(report.metrics.exact_position_rate, 1.0);
        assert_eq!(report.metrics.known_base_match_rate, 1.0);
        assert!(report.metrics.mean_program_compression >= 2.0);
        assert!(report.metrics.failure_classes.is_empty());
        assert!(
            report
                .cases
                .iter()
                .all(|case| case.recovered_program.is_some())
        );
        assert!(report.cases.iter().all(|case| case.target_descriptor_match));
        assert!(report.cases.iter().all(|case| {
            case.recovered_mesh
                .as_ref()
                .is_some_and(|mesh| mesh.validate().is_ok())
        }));
    }

    #[test]
    fn character_recovery_is_deterministic() {
        let first = run_known_base_character_recovery_gate(92);
        let second = run_known_base_character_recovery_gate(92);
        let third = run_known_base_character_recovery_gate(93);

        assert_eq!(first, second);
        assert_ne!(first, third);
    }

    #[test]
    fn recovery_uses_public_mesh_observation_not_case_id_answer_key() {
        let mut suite = known_base_character_recovery_suite(94);
        suite.inputs[0].case_id = "character.case.renamed".to_owned();

        let report = run_known_base_character_recovery_suite(&suite);

        assert!(report.accepted);
        assert!(report.cases[0].strict_success);
        assert!(report.cases[0].target_descriptor_match);
    }

    #[test]
    fn feature_inference_covers_required_character_variants() {
        let report = run_known_base_character_recovery_gate(95);
        let features = report
            .cases
            .iter()
            .map(|case| case.inferred_features.expect("features recovered"))
            .collect::<Vec<_>>();

        assert!(features.iter().any(|features| features.symmetric));
        assert!(features.iter().any(|features| features.asymmetric));
        assert!(features.iter().any(|features| features.posed));
        assert!(features.iter().any(|features| features.clothed));
        assert!(features.iter().any(|features| features.hair));
        assert!(features.iter().any(|features| features.topology_edited));
        assert!(features.iter().any(|features| {
            features.asymmetric
                && features.posed
                && features.clothed
                && features.hair
                && features.topology_edited
        }));
    }

    #[test]
    fn recovered_programs_use_known_base_without_target_mesh_payloads() {
        let suite = known_base_character_recovery_suite(96);
        let report = run_known_base_character_recovery_suite(&suite);
        let expected_base_fingerprint = base_topology_library().fingerprint().0;

        for case in &report.cases {
            let program = case
                .recovered_program
                .as_ref()
                .expect("strict success should include a recovered program");
            let input = suite
                .inputs
                .iter()
                .find(|input| input.case_id == case.case_id)
                .expect("matching public input");
            assert!(case.target_descriptor_match);
            assert!(
                mesh_descriptor_core_matches(
                    case.recovered_mesh.as_ref().expect("recovered mesh"),
                    &input.mesh
                ),
                "recovered descriptor should preserve public mesh evidence without depending on artifact ID"
            );
            assert_eq!(
                program.grammar_profile,
                GrammarProfile::StrictFromVersionedLibrary
            );
            assert_eq!(
                program
                    .base_topology
                    .as_ref()
                    .expect("known base reference")
                    .fingerprint,
                expected_base_fingerprint
            );
            assert!(program.operations.iter().all(|operation| {
                operation.payloads.iter().all(|payload| {
                    payload.kind == OperationPayloadKind::SemanticParameters
                        && payload.perturbation_valid
                })
            }));
            assert!(
                case.verification
                    .verification
                    .as_ref()
                    .expect("strict verification")
                    .literal_target_mesh_bytes
                    == 0
            );
            assert!(
                case.verification
                    .verification
                    .as_ref()
                    .expect("strict verification")
                    .residual_bytes
                    == 0
            );
        }
    }

    #[test]
    fn unknown_or_stale_character_mesh_reports_actionable_failure() {
        let mut suite = known_base_character_recovery_suite(97);
        suite.inputs[0].mesh.topology_fingerprint = "stale".to_owned();

        let report = run_known_base_character_recovery_suite(&suite);

        assert!(!report.accepted);
        let first = &report.cases[0];
        assert!(!first.strict_success);
        assert!(!first.target_descriptor_match);
        assert_eq!(first.matched_base_count, 0);
        assert!(
            first
                .verification
                .failures
                .iter()
                .all(StrictInverseFailure::is_actionable)
        );
        assert!(
            first
                .failure_classes
                .contains(&StrictInverseFailureClass::NumericalNonExactness)
        );
        assert!(first.verification.failures.iter().any(|failure| {
            failure.path == "input.mesh" && failure.detail.contains("artifact fingerprint")
        }));
    }

    #[test]
    fn recovery_ignores_public_mesh_artifact_id_for_acceptance() {
        let mut suite = known_base_character_recovery_suite(100);
        suite.inputs[0].mesh.id = "mesh.external.opaque-renamed".to_owned();
        suite.inputs[0].mesh.artifact_fingerprint =
            character_mesh_artifact_fingerprint(&suite.inputs[0].mesh);

        let report = run_known_base_character_recovery_suite(&suite);

        assert!(report.accepted);
        assert!(report.cases[0].strict_success);
        assert!(report.cases[0].target_descriptor_match);
        assert_ne!(
            report.cases[0]
                .recovered_mesh
                .as_ref()
                .expect("recovered mesh")
                .id,
            suite.inputs[0].mesh.id
        );
    }

    #[test]
    fn semantic_descriptor_mismatch_blocks_exact_recovery_even_when_artifact_is_fresh() {
        let mut suite = known_base_character_recovery_suite(101);
        suite.inputs[0].mesh.semantic_descriptor_fingerprint = "stale".to_owned();
        suite.inputs[0].mesh.artifact_fingerprint =
            character_mesh_artifact_fingerprint(&suite.inputs[0].mesh);

        let report = run_known_base_character_recovery_suite(&suite);

        assert!(!report.accepted);
        assert!(!report.cases[0].strict_success);
        assert!(!report.cases[0].target_descriptor_match);
        assert_eq!(report.cases[0].matched_base_count, 0);
    }

    #[test]
    fn fresh_artifact_with_wrong_topology_or_position_fingerprint_is_not_exact() {
        let mut suite = known_base_character_recovery_suite(103);
        suite.inputs[0].mesh.topology_fingerprint = "wrong-but-self-consistent".to_owned();
        suite.inputs[0].mesh.artifact_fingerprint =
            character_mesh_artifact_fingerprint(&suite.inputs[0].mesh);

        let topology_report = run_known_base_character_recovery_suite(&suite);

        assert!(!topology_report.accepted);
        assert!(!topology_report.cases[0].target_descriptor_match);
        assert!(
            topology_report.cases[0]
                .failure_classes
                .contains(&StrictInverseFailureClass::NumericalNonExactness)
        );

        let mut suite = known_base_character_recovery_suite(104);
        suite.inputs[0].mesh.canonical_position_fingerprint =
            "wrong-but-self-consistent".to_owned();
        suite.inputs[0].mesh.artifact_fingerprint =
            character_mesh_artifact_fingerprint(&suite.inputs[0].mesh);

        let position_report = run_known_base_character_recovery_suite(&suite);

        assert!(!position_report.accepted);
        assert!(!position_report.cases[0].target_descriptor_match);
        assert!(
            position_report.cases[0]
                .failure_classes
                .contains(&StrictInverseFailureClass::NumericalNonExactness)
        );
    }

    #[test]
    fn topology_edited_descriptor_without_operation_source_is_not_recovered() {
        let features = KnownBaseCharacterMeshFeatures {
            symmetric: true,
            asymmetric: false,
            posed: false,
            clothed: false,
            hair: false,
            topology_edited: true,
        };
        let signature = known_base_character_signature_for_features(features);
        let descriptor = known_base_character_descriptor_for_features(features);
        let mut mesh = CharacterMeshArtifact {
            id: "mesh.synthetic.invalid_topology_edit".to_owned(),
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
        mesh.artifact_fingerprint = character_mesh_artifact_fingerprint(&mesh);
        let suite = KnownBaseCharacterRecoverySuite {
            seed: 105,
            inputs: vec![KnownBaseCharacterRecoveryInput {
                case_id: "character.case.invalid_topology_edit".to_owned(),
                mesh,
            }],
        };

        let report = run_known_base_character_recovery_suite(&suite);

        assert!(!report.accepted);
        assert!(report.cases[0].inferred_features.is_none());
        assert!(!report.cases[0].target_descriptor_match);
        assert!(
            report.cases[0]
                .failure_classes
                .contains(&StrictInverseFailureClass::SelectionNotExpressible)
        );
    }

    #[test]
    fn descriptor_proof_rejects_semantically_wrong_recovered_program() {
        let suite = known_base_character_recovery_suite(102);
        let input = suite
            .inputs
            .iter()
            .find(|input| {
                infer_features_from_mesh(&input.mesh)
                    .features
                    .is_some_and(|features| features.clothed && features.topology_edited)
            })
            .expect("fixture includes clothed topology-edited case");
        let features = infer_features_from_mesh(&input.mesh)
            .features
            .expect("features recover");
        let mut program = recovered_program(features);
        let garment = program
            .operations
            .iter_mut()
            .find(|operation| operation.id.0 == "op.recovered_character.garment")
            .expect("clothed program has garment operation");
        garment.kind = ModelingOperationKind::Array;

        let proof = recovered_program_descriptor_proof(&program, &input.mesh, features);

        assert!(!proof.strict_descriptor_match);
    }

    #[test]
    fn runtime_rejection_is_reported_as_actionable_failure() {
        let runtime = ForwardProgramRuntimeReport {
            program_id: "runtime.test".to_owned(),
            accepted: false,
            operation_count: 1,
            trace_fingerprint: None,
            semantic_result_fingerprint: None,
            stage_provenance_complete: false,
            deterministic_replay: false,
            adapter_cache_separated: false,
            operations: Vec::new(),
            issues: vec![ForwardRuntimeIssue {
                code: ForwardRuntimeIssueCode::ReplayFailed,
                path: "runtime.test".to_owned(),
                message: "synthetic runtime rejection".to_owned(),
            }],
        };
        let mut failures = Vec::new();

        add_runtime_failures(Some(&runtime), &mut failures);

        assert_eq!(failures.len(), 1);
        assert_eq!(
            failures[0].class,
            StrictInverseFailureClass::UnsupportedSerializationOrder
        );
        assert!(failures[0].is_actionable());
    }

    #[test]
    fn suite_inputs_are_public_corpus_cases_only() {
        let suite = known_base_character_recovery_suite(98);
        let public = generated_character_corpus(98);

        assert_eq!(suite.inputs.len(), public.cases.len());
        for (input, public_case) in suite.inputs.iter().zip(public.cases.iter()) {
            assert_eq!(input.case_id, public_case.id);
            assert_eq!(input.mesh, public_case.mesh);
        }
    }

    #[test]
    fn public_case_type_is_mesh_only_for_recovery() {
        let public = generated_character_corpus(99);
        for case in public.cases {
            let ExposedCharacterBenchmarkCase { id, mesh } = case;
            assert!(id.starts_with("character.case."));
            mesh.validate().expect("public mesh should validate");
        }
    }

    fn mesh_descriptor_core_matches(
        left: &CharacterMeshArtifact,
        right: &CharacterMeshArtifact,
    ) -> bool {
        left.canonical_units == right.canonical_units
            && left.coordinate_system == right.coordinate_system
            && left.semantic_descriptor_fingerprint == right.semantic_descriptor_fingerprint
            && left.raw_geometry_size == right.raw_geometry_size
            && left.connected_component_count == right.connected_component_count
            && left.bounds_min == right.bounds_min
            && left.bounds_max == right.bounds_max
            && left.topology_fingerprint == right.topology_fingerprint
            && left.canonical_position_fingerprint == right.canonical_position_fingerprint
    }
}
