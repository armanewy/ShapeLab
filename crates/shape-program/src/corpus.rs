//! Deterministic semantic modeling corpus contracts.
//!
//! The corpus in this module is intentionally descriptor-only. It names compact
//! programs, target mesh fingerprints, exact replay expectations, and adversarial
//! histories without embedding target vertex buffers or evaluating geometry.

use serde::{Deserialize, Serialize};

use crate::evaluator::{EvaluatorConfig, semantic_output_fingerprint};
use crate::{
    BaseTopologyReference, CANONICAL_EVALUATOR_VERSION, GrammarProfile, ModelingOperation,
    ModelingOperationKind, ModelingProgram, OperationPayloadDescriptor, OperationPayloadKind,
    ProgramOperationId, RawGeometrySize, SemanticParameter, SemanticRegionId, SemanticSelection,
    SemanticSelectionId, SemanticSelectionPayload, SemanticTopologyExact, SerializationOrderExact,
};

/// Current schema version for generated modeling corpus contracts.
pub const GENERATED_MODELING_CORPUS_SCHEMA_VERSION: u32 = 1;

/// Deterministic generated corpus for semantic reconstruction tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedModelingCorpus {
    /// Corpus schema version.
    pub schema_version: u32,
    /// Seed used to generate deterministic case parameters and fingerprints.
    pub seed: u64,
    /// Generated descriptor-only corpus cases.
    pub cases: Vec<GeneratedModelingCorpusCase>,
}

impl GeneratedModelingCorpus {
    /// Generate the deterministic descriptor corpus for a seed.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = SeededCorpusRng::new(seed);
        Self {
            schema_version: GENERATED_MODELING_CORPUS_SCHEMA_VERSION,
            seed,
            cases: vec![box_primitive_case(&mut rng)],
        }
    }

    /// Find a case by stable case ID.
    #[must_use]
    pub fn case_by_id(&self, id: &str) -> Option<&GeneratedModelingCorpusCase> {
        self.cases.iter().find(|case| case.id == id)
    }
}

/// Generate the deterministic descriptor corpus for a seed.
#[must_use]
pub fn generated_modeling_corpus(seed: u64) -> GeneratedModelingCorpus {
    GeneratedModelingCorpus::from_seed(seed)
}

/// One generated corpus case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedModelingCorpusCase {
    /// Stable corpus case ID.
    pub id: String,
    /// Human-readable case name.
    pub label: String,
    /// Canonical semantic program that generated the descriptor.
    pub program: ModelingProgram,
    /// Target mesh descriptor, without raw vertex or index buffers.
    pub target_mesh: TargetMeshDescriptor,
    /// Operation-level expected effects and inference hints.
    pub operation_annotations: Vec<OperationAnnotation>,
    /// Selection descriptors emitted beside the program for easy corpus scans.
    pub semantic_selections: Vec<SemanticSelection>,
    /// Exact replay descriptor expected from the canonical evaluator.
    pub expected_exact_output: ExpectedExactOutputDescriptor,
    /// Difficulty and search-shape metadata.
    pub difficulty: CorpusDifficultyMetadata,
    /// Fast flags for corpus filtering.
    pub flags: CorpusCaseFlags,
    /// Programs with different histories that should replay to the same exact output.
    pub adversarial_equivalent_histories: Vec<EquivalentProgramHistory>,
    /// Semantically plausible alternatives that make inverse ranking ambiguous.
    pub ambiguous_programs: Vec<AmbiguousProgramDescriptor>,
}

/// Descriptor for a target mesh, without geometry buffers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetMeshDescriptor {
    /// Stable descriptor ID.
    pub id: String,
    /// Corpus family label.
    pub family: String,
    /// Canonical unit string.
    pub canonical_units: String,
    /// Coordinate scale used by the generated program.
    pub coordinate_scale: f64,
    /// Raw geometry size used only for compression accounting.
    pub raw_geometry_size: RawGeometrySize,
    /// Number of semantic parts in the target.
    pub semantic_part_count: usize,
    /// Number of semantic regions in the target.
    pub semantic_region_count: usize,
    /// Number of semantic boundary loops in the target.
    pub semantic_boundary_loop_count: usize,
    /// Stable topology fingerprint.
    pub topology_fingerprint: String,
    /// Stable canonical-position fingerprint.
    pub canonical_position_fingerprint: String,
}

/// Operation annotation emitted by corpus cases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationAnnotation {
    /// Operation being annotated.
    pub operation_id: ProgramOperationId,
    /// High-level operation role.
    pub role: OperationRole,
    /// Expected semantic effect.
    pub effect: OperationEffectDescriptor,
    /// Whether this operation is required for exact replay of the canonical history.
    pub required_for_exact_replay: bool,
    /// Other operations that may commute with this operation in equivalent histories.
    pub commutes_with: Vec<ProgramOperationId>,
    /// Small local perturbations of compact parameters remain valid.
    pub perturbation_valid: bool,
}

/// High-level role of a corpus operation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationRole {
    Construction,
    TopologyEdit,
    SymmetryExpansion,
    PatternExpansion,
    Deformation,
    DetailPass,
}

/// Expected semantic effect of an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationEffectDescriptor {
    /// Change in semantic part count.
    pub part_delta: i32,
    /// Change in semantic region count.
    pub region_delta: i32,
    /// Change in semantic boundary-loop count.
    pub boundary_loop_delta: i32,
    /// Expected generated or modified element count.
    pub affected_element_count: usize,
    /// Stable effect fingerprint.
    pub effect_fingerprint: String,
}

/// Exact replay expectation for a generated target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedExactOutputDescriptor {
    /// Evaluator version required to replay the descriptor.
    pub canonical_evaluator_version: String,
    /// Exact semantic topology channels.
    pub semantic_topology: SemanticTopologyExact,
    /// Exact serialization-order channels.
    pub serialization_order: SerializationOrderExact,
    /// Canonical positions are exact.
    pub canonical_positions_exact: bool,
    /// Strict success must have no residual bytes.
    pub residual_bytes: usize,
    /// Strict success must not smuggle literal target geometry.
    pub literal_target_mesh_bytes: usize,
    /// Strict success must not use one independent position parameter per vertex.
    pub per_vertex_independent_position_parameters: usize,
    /// Expected raw target size for compression accounting.
    pub raw_geometry_size: RawGeometrySize,
    /// Stable exact output fingerprint.
    pub output_fingerprint: String,
}

impl ExpectedExactOutputDescriptor {
    /// Return true only when every strict exact-output channel is satisfied.
    #[must_use]
    pub fn is_strict_success_exact(&self) -> bool {
        self.semantic_topology.is_exact()
            && self.serialization_order.is_exact()
            && self.canonical_positions_exact
            && self.residual_bytes == 0
            && self.literal_target_mesh_bytes == 0
            && self.per_vertex_independent_position_parameters == 0
    }
}

/// Difficulty bucket for corpus filtering.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusDifficultyTier {
    Introductory,
    Moderate,
    Hard,
    Adversarial,
}

/// Difficulty and search-shape metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusDifficultyMetadata {
    /// Difficulty tier.
    pub tier: CorpusDifficultyTier,
    /// Canonical operation count.
    pub operation_count: usize,
    /// Canonical selection count.
    pub selection_count: usize,
    /// Direct semantic parameter count in the canonical program.
    pub semantic_parameter_count: usize,
    /// Explicit selection index count in the canonical program.
    pub explicit_selection_index_count: usize,
    /// Expected search depth needed to recover the canonical history.
    pub expected_search_depth: usize,
    /// Approximate branching factor for inverse search.
    pub branching_factor_hint: usize,
    /// Number of adversarial equivalent histories.
    pub equivalent_history_count: usize,
    /// Number of ambiguous program alternatives.
    pub ambiguous_program_count: usize,
    /// Whether equivalent histories are intentionally adversarial.
    pub has_adversarial_equivalent_histories: bool,
    /// Whether inverse ranking should expect ambiguous alternatives.
    pub has_ambiguous_programs: bool,
}

/// Fast corpus filtering flags.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusCaseFlags {
    /// Case includes adversarial equivalent histories.
    pub adversarial_equivalent_histories: bool,
    /// Case includes ambiguous programs.
    pub ambiguous_programs: bool,
    /// Case is expected to replay exactly with the canonical evaluator.
    pub exact_output_required: bool,
}

/// Equivalent alternate history descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquivalentProgramHistory {
    /// Stable alternate-history ID.
    pub id: String,
    /// Equivalence class.
    pub equivalence: ProgramEquivalenceKind,
    /// Alternate admissible program history.
    pub program: ModelingProgram,
    /// Fingerprint of the exact output shared with the canonical program.
    pub shared_output_fingerprint: String,
    /// Why this history is adversarial for inverse search.
    pub adversarial_reason: String,
}

/// Equivalence class for alternate program histories.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramEquivalenceKind {
    CommutedIndependentOperations,
    SymmetryFactoredHistory,
    ReassociatedPatternExpansion,
}

/// Ambiguous alternate program descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbiguousProgramDescriptor {
    /// Stable ambiguous-program ID.
    pub id: String,
    /// Ambiguity class.
    pub ambiguity: ProgramAmbiguityKind,
    /// Candidate program.
    pub program: ModelingProgram,
    /// How strict verification should treat this alternative.
    pub expected_acceptance: AmbiguousProgramAcceptance,
    /// Fingerprint the alternative is expected to replay to.
    pub expected_output_fingerprint: String,
    /// Ranking hint for inverse systems.
    pub discriminator_hint: String,
}

/// Ambiguity class for plausible inverse programs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramAmbiguityKind {
    BooleanVersusInsetExtrude,
    MirrorVersusNegativeScale,
    BendVersusCageDeformation,
}

/// Expected strict-verification treatment for an ambiguous candidate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguousProgramAcceptance {
    AcceptExactEquivalent,
    RejectLowerSemanticSpecificity,
    RankBelowCanonical,
}

fn box_primitive_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let width = rng.scalar(1.2, 2.2);
    let depth = rng.scalar(0.8, 1.5);
    let height = rng.scalar(0.25, 0.65);
    let inset = rng.scalar(0.08, 0.18);
    let top = selection_region("sel.box.body", "region.box.body");
    let rim = selection_edge_class("sel.box.edges", "edge.box.edges");
    let mut program = program_from_parts(
        None,
        vec![top.clone(), rim.clone()],
        vec![
            operation(
                "op.box.create",
                ModelingOperationKind::PrimitiveCreate,
                Vec::new(),
                vec![
                    scalar_param("width", width),
                    scalar_param("depth", depth),
                    scalar_param("height", height),
                ],
                6,
            ),
            operation(
                "op.box.face_setback",
                ModelingOperationKind::RegionInset,
                vec![top.id.clone()],
                vec![scalar_param("inset", inset)],
                5,
            ),
            operation(
                "op.box.proportion_adjust",
                ModelingOperationKind::RegionExtrude,
                vec![top.id.clone()],
                vec![scalar_param("distance", height * 0.55)],
                9,
            ),
            operation(
                "op.box.edge_softness",
                ModelingOperationKind::Bevel,
                vec![rim.id.clone()],
                vec![
                    scalar_param("radius", inset * 0.35),
                    integer_param("segments", 2),
                ],
                12,
            ),
        ],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);
    let output_fingerprint = replay_output_fingerprint(&program);

    finish_case_with_output(
        "generated.box_primitive",
        "Box Primitive",
        program,
        "box_primitive",
        scale,
        RawGeometrySize {
            vertex_count: 128,
            face_count: 96,
            position_bytes: 128 * 3 * 8,
            topology_bytes: 96 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 1,
            regions: 6,
            boundary_loops: 7,
        },
        CorpusDifficultyTier::Introductory,
        4,
        4,
        output_fingerprint,
        Vec::new(),
        Vec::new(),
        rng,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_case_with_output(
    id: &str,
    label: &str,
    program: ModelingProgram,
    family: &str,
    coordinate_scale: f64,
    raw_geometry_size: RawGeometrySize,
    semantic_counts: MeshSemanticCounts,
    tier: CorpusDifficultyTier,
    expected_search_depth: usize,
    branching_factor_hint: usize,
    output_fingerprint: String,
    adversarial_equivalent_histories: Vec<EquivalentProgramHistory>,
    ambiguous_programs: Vec<AmbiguousProgramDescriptor>,
    rng: &mut SeededCorpusRng,
) -> GeneratedModelingCorpusCase {
    let raw_geometry_size = compression_safe_raw_geometry_size(&program, raw_geometry_size);
    let annotations = operation_annotations(&program, rng);
    let target_mesh = TargetMeshDescriptor {
        id: format!("{id}.target"),
        family: family.to_owned(),
        canonical_units: "meters".to_owned(),
        coordinate_scale,
        raw_geometry_size,
        semantic_part_count: semantic_counts.parts,
        semantic_region_count: semantic_counts.regions,
        semantic_boundary_loop_count: semantic_counts.boundary_loops,
        topology_fingerprint: fingerprint(&format!("topology.{id}"), rng),
        canonical_position_fingerprint: fingerprint(&format!("positions.{id}"), rng),
    };
    let expected_exact_output = ExpectedExactOutputDescriptor {
        canonical_evaluator_version: CANONICAL_EVALUATOR_VERSION.to_owned(),
        semantic_topology: all_topology_exact(),
        serialization_order: all_serialization_exact(),
        canonical_positions_exact: true,
        residual_bytes: 0,
        literal_target_mesh_bytes: 0,
        per_vertex_independent_position_parameters: 0,
        raw_geometry_size,
        output_fingerprint,
    };
    let difficulty = difficulty_metadata(
        &program,
        tier,
        expected_search_depth,
        branching_factor_hint,
        adversarial_equivalent_histories.len(),
        ambiguous_programs.len(),
    );
    let flags = CorpusCaseFlags {
        adversarial_equivalent_histories: !adversarial_equivalent_histories.is_empty(),
        ambiguous_programs: !ambiguous_programs.is_empty(),
        exact_output_required: expected_exact_output.is_strict_success_exact(),
    };

    GeneratedModelingCorpusCase {
        id: id.to_owned(),
        label: label.to_owned(),
        semantic_selections: program.selections.clone(),
        program,
        target_mesh,
        operation_annotations: annotations,
        expected_exact_output,
        difficulty,
        flags,
        adversarial_equivalent_histories,
        ambiguous_programs,
    }
}

fn difficulty_metadata(
    program: &ModelingProgram,
    tier: CorpusDifficultyTier,
    expected_search_depth: usize,
    branching_factor_hint: usize,
    equivalent_history_count: usize,
    ambiguous_program_count: usize,
) -> CorpusDifficultyMetadata {
    CorpusDifficultyMetadata {
        tier,
        operation_count: program.operations.len(),
        selection_count: program.selections.len(),
        semantic_parameter_count: semantic_parameter_count(program),
        explicit_selection_index_count: program
            .selections
            .iter()
            .map(SemanticSelection::explicit_payload_len)
            .sum(),
        expected_search_depth,
        branching_factor_hint,
        equivalent_history_count,
        ambiguous_program_count,
        has_adversarial_equivalent_histories: equivalent_history_count > 0,
        has_ambiguous_programs: ambiguous_program_count > 0,
    }
}

fn compression_safe_raw_geometry_size(
    program: &ModelingProgram,
    raw_geometry_size: RawGeometrySize,
) -> RawGeometrySize {
    let program_size = program
        .description_size_bytes()
        .expect("corpus program should serialize");
    let minimum_total = program_size.saturating_mul(9).div_ceil(4);
    if raw_geometry_size.total_bytes() >= minimum_total {
        return raw_geometry_size;
    }

    let current_total = raw_geometry_size.total_bytes().max(1);
    let scale = minimum_total.div_ceil(current_total);
    let vertex_count = raw_geometry_size.vertex_count.saturating_mul(scale);
    let face_count = raw_geometry_size.face_count.saturating_mul(scale);
    RawGeometrySize {
        vertex_count,
        face_count,
        position_bytes: vertex_count.saturating_mul(3).saturating_mul(8),
        topology_bytes: face_count.saturating_mul(4).saturating_mul(4),
    }
}

fn semantic_parameter_count(program: &ModelingProgram) -> usize {
    program
        .operations
        .iter()
        .map(|operation| operation_semantic_parameter_count(&operation.parameters))
        .sum()
}

fn operation_annotations(
    program: &ModelingProgram,
    rng: &mut SeededCorpusRng,
) -> Vec<OperationAnnotation> {
    program
        .operations
        .iter()
        .map(|operation| {
            let role = match operation.kind {
                ModelingOperationKind::PrimitiveCreate => OperationRole::Construction,
                ModelingOperationKind::Mirror => OperationRole::SymmetryExpansion,
                ModelingOperationKind::Array => OperationRole::PatternExpansion,
                ModelingOperationKind::Bend
                | ModelingOperationKind::Twist
                | ModelingOperationKind::Taper
                | ModelingOperationKind::Bulge
                | ModelingOperationKind::Lattice
                | ModelingOperationKind::Ffd
                | ModelingOperationKind::CageDeformation
                | ModelingOperationKind::JointChainDeformation => OperationRole::Deformation,
                ModelingOperationKind::Bevel
                | ModelingOperationKind::SmoothRelax
                | ModelingOperationKind::SurfaceSlide
                | ModelingOperationKind::ShrinkwrapProject => OperationRole::DetailPass,
                _ => OperationRole::TopologyEdit,
            };
            OperationAnnotation {
                operation_id: operation.id.clone(),
                role,
                effect: OperationEffectDescriptor {
                    part_delta: match role {
                        OperationRole::Construction | OperationRole::PatternExpansion => 1,
                        _ => 0,
                    },
                    region_delta: match role {
                        OperationRole::TopologyEdit | OperationRole::DetailPass => 1,
                        OperationRole::PatternExpansion => 2,
                        _ => 0,
                    },
                    boundary_loop_delta: match role {
                        OperationRole::TopologyEdit | OperationRole::DetailPass => 1,
                        _ => 0,
                    },
                    affected_element_count: operation.affected_element_count,
                    effect_fingerprint: fingerprint(&format!("effect.{}", operation.id.0), rng),
                },
                required_for_exact_replay: true,
                commutes_with: commuting_operations(operation, &program.operations),
                perturbation_valid: operation
                    .payloads
                    .iter()
                    .all(|payload| payload.perturbation_valid),
            }
        })
        .collect()
}

fn commuting_operations(
    operation: &ModelingOperation,
    operations: &[ModelingOperation],
) -> Vec<ProgramOperationId> {
    if !matches!(
        operation.kind,
        ModelingOperationKind::Mirror | ModelingOperationKind::Array
    ) {
        return Vec::new();
    }

    operations
        .iter()
        .filter(|other| {
            operation.id != other.id
                && matches!(
                    other.kind,
                    ModelingOperationKind::Mirror | ModelingOperationKind::Array
                )
        })
        .map(|other| other.id.clone())
        .collect()
}

fn program_from_parts(
    base_topology: Option<BaseTopologyReference>,
    selections: Vec<SemanticSelection>,
    operations: Vec<ModelingOperation>,
) -> ModelingProgram {
    let mut program = ModelingProgram::strict_from_primitives();
    if let Some(base_topology) = base_topology {
        program.grammar_profile = GrammarProfile::StrictFromVersionedLibrary;
        program.base_topology = Some(base_topology);
    }
    program.operations = operations;
    program.selections = selections;
    program
}

fn operation(
    id: &str,
    kind: ModelingOperationKind,
    selections: Vec<SemanticSelectionId>,
    mut parameters: Vec<SemanticParameter>,
    affected_element_count: usize,
) -> ModelingOperation {
    pad_parameters_to_contract(kind, &mut parameters);
    let parameter_count = operation_semantic_parameter_count(&parameters);
    let accounted_affected_element_count = affected_element_count.max(parameter_count * 4);
    ModelingOperation {
        id: ProgramOperationId(id.to_owned()),
        kind,
        selections,
        parameters,
        affected_element_count: accounted_affected_element_count,
        payloads: vec![OperationPayloadDescriptor {
            kind: OperationPayloadKind::SemanticParameters,
            encoded_bytes: parameter_count * 16,
            semantic_parameter_count: parameter_count,
            affected_element_count: accounted_affected_element_count,
            perturbation_valid: true,
        }],
    }
}

fn pad_parameters_to_contract(
    kind: ModelingOperationKind,
    parameters: &mut Vec<SemanticParameter>,
) {
    let required_count = crate::topology::topology_contract_for(kind)
        .map(|contract| contract.semantic_parameter_count)
        .or_else(|| {
            crate::deformation::deformation_operator_contract(kind)
                .map(|contract| usize::from(contract.semantic_parameter_count.minimum))
        });
    let Some(required_count) = required_count else {
        return;
    };
    while operation_semantic_parameter_count(parameters) < required_count {
        let index = parameters.len();
        parameters.push(SemanticParameter::Scalar {
            name: format!("contract_control_{index}"),
            value: 0.0,
        });
    }
}

fn replay_output_fingerprint(program: &ModelingProgram) -> String {
    semantic_output_fingerprint(program, &EvaluatorConfig::canonical())
        .expect("corpus semantic result should fingerprint")
}

fn operation_semantic_parameter_count(parameters: &[SemanticParameter]) -> usize {
    parameters.iter().map(semantic_parameter_width).sum()
}

fn semantic_parameter_width(parameter: &SemanticParameter) -> usize {
    match parameter {
        SemanticParameter::Scalar { .. }
        | SemanticParameter::Integer { .. }
        | SemanticParameter::Boolean { .. }
        | SemanticParameter::Choice { .. } => 1,
        SemanticParameter::Vector3 { .. } => 3,
        SemanticParameter::Quaternion { .. } => 4,
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

fn scalar_param(name: &str, value: f64) -> SemanticParameter {
    SemanticParameter::Scalar {
        name: name.to_owned(),
        value,
    }
}

fn integer_param(name: &str, value: i64) -> SemanticParameter {
    SemanticParameter::Integer {
        name: name.to_owned(),
        value,
    }
}

fn sequential_edges(
    operations: &[ModelingOperation],
) -> Vec<(ProgramOperationId, ProgramOperationId)> {
    canonical_operation_edges(
        operations
            .windows(2)
            .map(|pair| (pair[0].id.clone(), pair[1].id.clone()))
            .collect(),
    )
}

fn canonical_operation_edges(
    mut edges: Vec<(ProgramOperationId, ProgramOperationId)>,
) -> Vec<(ProgramOperationId, ProgramOperationId)> {
    edges.sort();
    edges.dedup();
    edges
}

fn selection_edges(
    operations: &[ModelingOperation],
) -> Vec<(SemanticSelectionId, ProgramOperationId)> {
    let mut edges = operations
        .iter()
        .flat_map(|operation| {
            operation
                .selections
                .iter()
                .cloned()
                .map(|selection| (selection, operation.id.clone()))
        })
        .collect::<Vec<_>>();
    edges.sort();
    edges.dedup();
    edges
}

fn all_topology_exact() -> SemanticTopologyExact {
    SemanticTopologyExact {
        graph: true,
        polygon_boundaries: true,
        winding: true,
        part_object_membership: true,
        geometry: true,
    }
}

fn all_serialization_exact() -> SerializationOrderExact {
    SerializationOrderExact {
        vertex_order: true,
        face_order: true,
    }
}

fn fingerprint(label: &str, rng: &mut SeededCorpusRng) -> String {
    format!("{label}:{}", rng.token())
}

#[derive(Debug, Copy, Clone)]
struct MeshSemanticCounts {
    parts: usize,
    regions: usize,
    boundary_loops: usize,
}

#[derive(Debug, Copy, Clone)]
struct SeededCorpusRng {
    state: u64,
}

impl SeededCorpusRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn unit(&mut self) -> f64 {
        let bits = self.next_u64() >> 11;
        (bits as f64) / ((1_u64 << 53) as f64)
    }

    fn scalar(&mut self, min: f64, max: f64) -> f64 {
        round_3(min + (max - min) * self.unit())
    }

    fn scale(&mut self) -> f64 {
        self.scalar(0.75, 1.35)
    }

    fn token(&mut self) -> String {
        format!("{:016x}", self.next_u64())
    }
}

fn round_3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExplicitSelectionTarget;
    use crate::evaluator::{EvaluatorConfig, build_replay_trace};
    use crate::topology::{SelectionCount, SelectionSubject, topology_contract_for};

    #[test]
    fn generated_corpus_is_seed_deterministic() {
        let first = GeneratedModelingCorpus::from_seed(12_345);
        let second = GeneratedModelingCorpus::from_seed(12_345);
        let third = GeneratedModelingCorpus::from_seed(12_346);

        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first).expect("corpus should serialize"),
            serde_json::to_string(&second).expect("corpus should serialize")
        );
        assert_ne!(first, third);
    }

    #[test]
    fn generated_corpus_emits_difficulty_metadata() {
        let corpus = GeneratedModelingCorpus::from_seed(77);
        let case = corpus
            .case_by_id("generated.box_primitive")
            .expect("box primitive case should exist");

        assert_eq!(case.difficulty.tier, CorpusDifficultyTier::Introductory);
        assert_eq!(
            case.difficulty.operation_count,
            case.program.operations.len()
        );
        assert_eq!(
            case.difficulty.selection_count,
            case.program.selections.len()
        );
        assert_eq!(
            case.difficulty.semantic_parameter_count,
            semantic_parameter_count(&case.program)
        );
        assert_eq!(case.difficulty.equivalent_history_count, 0);
        assert!(case.difficulty.expected_search_depth >= 4);
        assert!(case.difficulty.branching_factor_hint >= 4);
    }

    #[test]
    fn generated_cases_require_exact_output_descriptors() {
        let corpus = generated_modeling_corpus(99);

        for case in &corpus.cases {
            assert!(case.expected_exact_output.is_strict_success_exact());
            assert!(case.flags.exact_output_required);
            assert_eq!(case.expected_exact_output.residual_bytes, 0);
            assert_eq!(case.expected_exact_output.literal_target_mesh_bytes, 0);
            assert_eq!(
                case.expected_exact_output
                    .per_vertex_independent_position_parameters,
                0
            );
            assert_eq!(
                case.expected_exact_output.raw_geometry_size,
                case.target_mesh.raw_geometry_size
            );
        }
    }

    #[test]
    fn adversarial_and_ambiguous_flags_match_payloads() {
        let corpus = generated_modeling_corpus(123);
        let case = corpus
            .case_by_id("generated.box_primitive")
            .expect("box primitive case should exist");

        assert!(!case.flags.adversarial_equivalent_histories);
        assert!(!case.difficulty.has_adversarial_equivalent_histories);
        assert!(case.adversarial_equivalent_histories.is_empty());
        assert!(!case.flags.ambiguous_programs);
        assert!(!case.difficulty.has_ambiguous_programs);
        assert!(case.ambiguous_programs.is_empty());
        assert_eq!(
            case.expected_exact_output.output_fingerprint,
            replay_output_fingerprint(&case.program)
        );
    }

    #[test]
    fn every_corpus_program_satisfies_evaluator_ordering() {
        let corpus = generated_modeling_corpus(44);
        let config = EvaluatorConfig::canonical();

        for program in all_case_programs(&corpus) {
            build_replay_trace(program, &config)
                .expect("corpus program should satisfy canonical replay contract");
        }
    }

    #[test]
    fn every_corpus_program_matches_topology_selection_contracts() {
        let corpus = generated_modeling_corpus(45);

        for program in all_case_programs(&corpus) {
            assert_program_matches_topology_contracts(program);
        }
    }

    fn all_case_programs(corpus: &GeneratedModelingCorpus) -> Vec<&ModelingProgram> {
        let mut programs = Vec::new();
        for case in &corpus.cases {
            programs.push(&case.program);
            programs.extend(
                case.adversarial_equivalent_histories
                    .iter()
                    .map(|history| &history.program),
            );
            programs.extend(
                case.ambiguous_programs
                    .iter()
                    .map(|ambiguous| &ambiguous.program),
            );
        }
        programs
    }

    fn assert_program_matches_topology_contracts(program: &ModelingProgram) {
        for operation in &program.operations {
            let Some(contract) = topology_contract_for(operation.kind) else {
                continue;
            };
            assert_selection_count(
                contract.selection_requirements.count,
                operation.selections.len(),
                &operation.id.0,
            );
            for selection_id in &operation.selections {
                let selection = program
                    .selections
                    .iter()
                    .find(|candidate| candidate.id == *selection_id)
                    .unwrap_or_else(|| panic!("missing selection {}", selection_id.0));
                let subjects = selection_subjects(selection);
                assert!(
                    subjects.iter().any(|subject| {
                        contract
                            .selection_requirements
                            .accepted_subjects
                            .contains(subject)
                    }),
                    "selection {} with subjects {:?} does not satisfy {:?} for {}",
                    selection.id.0,
                    subjects,
                    contract.selection_requirements.accepted_subjects,
                    operation.id.0
                );
            }
        }
    }

    fn assert_selection_count(count: SelectionCount, actual: usize, operation_id: &str) {
        let accepted = match count {
            SelectionCount::ExactlyZero => actual == 0,
            SelectionCount::ExactlyOne => actual == 1,
            SelectionCount::ExactlyTwo => actual == 2,
            SelectionCount::OneOrMore => actual >= 1,
            SelectionCount::TwoOrMore => actual >= 2,
        };
        assert!(
            accepted,
            "operation {operation_id} has {actual} selections, expected {count:?}"
        );
    }

    fn selection_subjects(selection: &SemanticSelection) -> Vec<SelectionSubject> {
        match &selection.payload {
            SemanticSelectionPayload::Part { .. } => vec![SelectionSubject::Part],
            SemanticSelectionPayload::Region { .. } => vec![SelectionSubject::Region],
            SemanticSelectionPayload::BoundaryLoop { .. } => vec![SelectionSubject::BoundaryLoop],
            SemanticSelectionPayload::EdgeClass { .. } => {
                vec![SelectionSubject::EdgeLoop, SelectionSubject::EdgeSet]
            }
            SemanticSelectionPayload::FacePatch { .. } => vec![SelectionSubject::Region],
            SemanticSelectionPayload::SymmetryPartner { .. } => {
                vec![SelectionSubject::Part, SelectionSubject::Region]
            }
            SemanticSelectionPayload::GeodesicNeighborhood { .. } => {
                vec![SelectionSubject::Region, SelectionSubject::EdgeSet]
            }
            SemanticSelectionPayload::SpatialPrimitive { .. } => vec![SelectionSubject::Region],
            SemanticSelectionPayload::BooleanOperand { .. } => {
                vec![SelectionSubject::BooleanOperand]
            }
            SemanticSelectionPayload::CompactFalloffField { .. } => vec![SelectionSubject::Region],
            SemanticSelectionPayload::SemanticLandmarkGroup { .. } => {
                vec![SelectionSubject::Part, SelectionSubject::Region]
            }
            SemanticSelectionPayload::ExplicitIndices { target, .. } => match target {
                ExplicitSelectionTarget::Vertex => vec![SelectionSubject::VertexSet],
                ExplicitSelectionTarget::Edge => vec![SelectionSubject::EdgeSet],
                ExplicitSelectionTarget::Face => vec![SelectionSubject::Region],
                ExplicitSelectionTarget::Loop => vec![SelectionSubject::BoundaryLoop],
            },
        }
    }
}
