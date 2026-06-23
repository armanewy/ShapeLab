//! Deterministic semantic modeling corpus contracts.
//!
//! The corpus in this module is intentionally descriptor-only. It names compact
//! programs, target mesh fingerprints, exact replay expectations, and adversarial
//! histories without embedding target vertex buffers or evaluating geometry.

use serde::{Deserialize, Serialize};

use crate::evaluator::{EvaluatorConfig, semantic_output_fingerprint};
use crate::{
    BaseTopologyReference, CANONICAL_EVALUATOR_VERSION, ExplicitSelectionTarget, GrammarProfile,
    ModelingOperation, ModelingOperationKind, ModelingProgram, OperationPayloadDescriptor,
    OperationPayloadKind, ProgramOperationId, RawGeometrySize, SemanticParameter, SemanticPartId,
    SemanticRegionId, SemanticSelection, SemanticSelectionId, SemanticSelectionPayload,
    SemanticTopologyExact, SerializationOrderExact, SpatialPrimitiveSelection,
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
            cases: vec![
                panel_extrude_case(&mut rng),
                mirrored_array_bracket_case(&mut rng),
                ambiguous_keyway_case(&mut rng),
                mechanical_tool_cart_case(&mut rng),
                furniture_workshop_stool_case(&mut rng),
                modular_wall_segment_case(&mut rng),
                stylized_organic_cactus_case(&mut rng),
                humanoid_blockout_case(&mut rng),
            ],
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

fn panel_extrude_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let width = rng.scalar(1.2, 2.2);
    let depth = rng.scalar(0.8, 1.5);
    let height = rng.scalar(0.25, 0.65);
    let inset = rng.scalar(0.08, 0.18);
    let top = selection_region("sel.panel.top", "region.panel.top");
    let rim = selection_edge_class("sel.panel.rim", "edge.panel.rim");
    let mut program = program_from_parts(
        None,
        vec![top.clone(), rim.clone()],
        vec![
            operation(
                "op.panel.create",
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
                "op.panel.inset",
                ModelingOperationKind::RegionInset,
                vec![top.id.clone()],
                vec![scalar_param("inset", inset)],
                5,
            ),
            operation(
                "op.panel.extrude",
                ModelingOperationKind::RegionExtrude,
                vec![top.id.clone()],
                vec![scalar_param("distance", height * 0.55)],
                9,
            ),
            operation(
                "op.panel.bevel",
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

    finish_case(
        "generated.panel_extrude",
        "Seeded panel extrude",
        program,
        "panel_extrude",
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
        Vec::new(),
        Vec::new(),
        rng,
    )
}

fn mirrored_array_bracket_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let base = selection_part("sel.bracket.base", "part.bracket.base");
    let side = selection_region("sel.bracket.side", "region.bracket.side");
    let seam = selection_edge_class("sel.bracket.seam", "edge.bracket.seam");
    let create = operation(
        "op.bracket.create",
        ModelingOperationKind::PrimitiveCreate,
        Vec::new(),
        vec![
            scalar_param("width", rng.scalar(0.7, 1.1)),
            scalar_param("height", rng.scalar(1.0, 1.8)),
            scalar_param("depth", rng.scalar(0.35, 0.6)),
        ],
        8,
    );
    let extrude = operation(
        "op.bracket.side_extrude",
        ModelingOperationKind::RegionExtrude,
        vec![side.id.clone()],
        vec![scalar_param("distance", rng.scalar(0.25, 0.45))],
        8,
    );
    let mirror = operation(
        "op.bracket.mirror",
        ModelingOperationKind::Mirror,
        vec![base.id.clone()],
        vec![choice_param("axis", "x"), boolean_param("weld", true)],
        18,
    );
    let array = operation(
        "op.bracket.array",
        ModelingOperationKind::Array,
        vec![base.id.clone()],
        vec![
            integer_param("count", 3),
            vector_param("step", [rng.scalar(0.35, 0.5), 0.0, 0.0]),
        ],
        36,
    );
    let bevel = operation(
        "op.bracket.bevel",
        ModelingOperationKind::Bevel,
        vec![seam.id.clone()],
        vec![scalar_param("radius", rng.scalar(0.025, 0.05))],
        24,
    );
    let selections = vec![base.clone(), side, seam];
    let mut program = program_from_parts(
        None,
        selections,
        vec![
            create.clone(),
            extrude.clone(),
            mirror.clone(),
            array.clone(),
            bevel.clone(),
        ],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);

    let mut equivalent = program_from_parts(
        None,
        program.selections.clone(),
        vec![create, extrude, array, mirror, bevel],
    );
    equivalent.dependency_graph.operation_edges = canonical_operation_edges(vec![
        edge("op.bracket.create", "op.bracket.side_extrude"),
        edge("op.bracket.side_extrude", "op.bracket.array"),
        edge("op.bracket.array", "op.bracket.mirror"),
        edge("op.bracket.mirror", "op.bracket.bevel"),
    ]);
    equivalent.dependency_graph.selection_edges = selection_edges(&equivalent.operations);

    let output_fingerprint = replay_output_fingerprint(&program);
    let equivalent_histories = vec![EquivalentProgramHistory {
        id: "equiv.bracket.array_before_mirror".to_owned(),
        equivalence: ProgramEquivalenceKind::CommutedIndependentOperations,
        program: equivalent,
        shared_output_fingerprint: output_fingerprint.clone(),
        adversarial_reason:
            "array and mirror produce the same welded lattice when the source part is symmetric"
                .to_owned(),
    }];

    finish_case_with_output(
        "generated.mirrored_array_bracket",
        "Mirrored array bracket",
        program,
        "mirrored_array_bracket",
        scale,
        RawGeometrySize {
            vertex_count: 156,
            face_count: 112,
            position_bytes: 156 * 3 * 8,
            topology_bytes: 112 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 3,
            regions: 18,
            boundary_loops: 24,
        },
        CorpusDifficultyTier::Hard,
        6,
        8,
        output_fingerprint,
        equivalent_histories,
        Vec::new(),
        rng,
    )
}

fn ambiguous_keyway_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let pocket = selection_region("sel.keyway.pocket", "region.keyway.pocket");
    let loop_path = selection_edge_class("sel.keyway.loop_path", "edge_loop.keyway.pocket");
    let host_part = selection_part("sel.keyway.host", "part.keyway.panel");
    let slot_operand = selection_boolean_operand("sel.keyway.slot_operand", "operand.keyway.slot");
    let slot_shape =
        selection_spatial_box("sel.keyway.slot_shape", [-0.2, -0.1, 0.0], [0.2, 0.1, 0.4]);
    let indices = selection_indices("sel.keyway.audit.indices", ExplicitSelectionTarget::Face, 6);
    let create = operation(
        "op.keyway.create",
        ModelingOperationKind::PrimitiveCreate,
        Vec::new(),
        vec![
            scalar_param("width", rng.scalar(1.0, 1.4)),
            scalar_param("height", rng.scalar(0.8, 1.1)),
            scalar_param("depth", rng.scalar(0.55, 0.8)),
        ],
        6,
    );
    let loop_cut = operation(
        "op.keyway.loop_cut",
        ModelingOperationKind::LoopCut,
        vec![loop_path.id.clone()],
        vec![scalar_param("offset", rng.scalar(0.25, 0.35))],
        12,
    );
    let inset = operation(
        "op.keyway.inset",
        ModelingOperationKind::RegionInset,
        vec![pocket.id.clone()],
        vec![scalar_param("inset", rng.scalar(0.04, 0.07))],
        8,
    );
    let extrude = operation(
        "op.keyway.recess",
        ModelingOperationKind::RegionExtrude,
        vec![pocket.id.clone()],
        vec![scalar_param("distance", -rng.scalar(0.08, 0.16))],
        10,
    );
    let selections = vec![
        pocket.clone(),
        loop_path,
        host_part.clone(),
        slot_operand.clone(),
        slot_shape,
        indices.clone(),
    ];
    let mut program = program_from_parts(
        Some(BaseTopologyReference {
            catalog_id: "mechanical-panel-base".to_owned(),
            version: "1.0".to_owned(),
            fingerprint: fingerprint("base.keyway", rng),
        }),
        selections,
        vec![create.clone(), loop_cut, inset, extrude],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);

    let boolean = operation(
        "op.keyway.boolean_slot",
        ModelingOperationKind::ConstrainedBoolean,
        vec![host_part.id.clone(), slot_operand.id.clone()],
        vec![
            choice_param("operation", "subtract"),
            scalar_param("clearance", rng.scalar(0.01, 0.025)),
        ],
        16,
    );
    let mut boolean_program = program_from_parts(
        program.base_topology.clone(),
        vec![pocket, host_part, slot_operand, indices],
        vec![create, boolean],
    );
    boolean_program.dependency_graph.operation_edges =
        sequential_edges(&boolean_program.operations);
    boolean_program.dependency_graph.selection_edges = selection_edges(&boolean_program.operations);

    let output_fingerprint = replay_output_fingerprint(&program);
    let ambiguous_output_fingerprint = replay_output_fingerprint(&boolean_program);
    let ambiguous = vec![AmbiguousProgramDescriptor {
        id: "ambiguous.keyway.boolean_subtract".to_owned(),
        ambiguity: ProgramAmbiguityKind::BooleanVersusInsetExtrude,
        program: boolean_program,
        expected_acceptance: AmbiguousProgramAcceptance::RankBelowCanonical,
        expected_output_fingerprint: ambiguous_output_fingerprint,
        discriminator_hint:
            "prefer the history with reusable pocket-region semantics when both replay exactly"
                .to_owned(),
    }];

    finish_case_with_output(
        "generated.ambiguous_keyway",
        "Ambiguous keyway",
        program,
        "ambiguous_keyway",
        scale,
        RawGeometrySize {
            vertex_count: 180,
            face_count: 140,
            position_bytes: 180 * 3 * 8,
            topology_bytes: 140 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 1,
            regions: 9,
            boundary_loops: 11,
        },
        CorpusDifficultyTier::Adversarial,
        7,
        9,
        output_fingerprint,
        Vec::new(),
        ambiguous,
        rng,
    )
}

fn mechanical_tool_cart_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let body = selection_part("sel.tool_cart.body", "part.tool_cart.body");
    let front_panel = selection_region("sel.tool_cart.front_panel", "region.tool_cart.front");
    let side_handle = selection_part("sel.tool_cart.side_handle", "part.tool_cart.handle");
    let rim = selection_edge_class("sel.tool_cart.rim", "edge.tool_cart.rim");
    let create = operation(
        "op.tool_cart.create",
        ModelingOperationKind::PrimitiveCreate,
        Vec::new(),
        vec![
            scalar_param("width", rng.scalar(0.8, 1.2)),
            scalar_param("height", rng.scalar(0.5, 0.8)),
            scalar_param("depth", rng.scalar(0.35, 0.6)),
        ],
        14,
    );
    let inset = operation(
        "op.tool_cart.drawer_inset",
        ModelingOperationKind::RegionInset,
        vec![front_panel.id.clone()],
        vec![scalar_param("inset", rng.scalar(0.025, 0.045))],
        12,
    );
    let recess = operation(
        "op.tool_cart.drawer_recess",
        ModelingOperationKind::RegionExtrude,
        vec![front_panel.id.clone()],
        vec![scalar_param("distance", -rng.scalar(0.02, 0.05))],
        16,
    );
    let mirror = operation(
        "op.tool_cart.mirror_handle",
        ModelingOperationKind::Mirror,
        vec![side_handle.id.clone()],
        vec![choice_param("axis", "x"), boolean_param("weld", false)],
        20,
    );
    let array = operation(
        "op.tool_cart.drawer_array",
        ModelingOperationKind::Array,
        vec![body.id.clone()],
        vec![
            integer_param("count", 3),
            vector_param("step", [0.0, -rng.scalar(0.12, 0.18), 0.0]),
        ],
        36,
    );
    let bevel = operation(
        "op.tool_cart.bevel_rim",
        ModelingOperationKind::Bevel,
        vec![rim.id.clone()],
        vec![scalar_param("radius", rng.scalar(0.015, 0.035))],
        28,
    );
    let mut program = program_from_parts(
        None,
        vec![body, front_panel, side_handle, rim],
        vec![create, inset, recess, mirror, array, bevel],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);

    finish_case(
        "generated.mechanical_tool_cart",
        "Mechanical tool cart",
        program,
        "mechanical_props",
        scale,
        RawGeometrySize {
            vertex_count: 188,
            face_count: 136,
            position_bytes: 188 * 3 * 8,
            topology_bytes: 136 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 5,
            regions: 22,
            boundary_loops: 30,
        },
        CorpusDifficultyTier::Moderate,
        6,
        9,
        Vec::new(),
        Vec::new(),
        rng,
    )
}

fn furniture_workshop_stool_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let seat_top = selection_region("sel.stool.seat_top", "region.stool.seat_top");
    let leg = selection_part("sel.stool.leg", "part.stool.leg");
    let back = selection_region("sel.stool.back", "region.stool.back");
    let rim = selection_edge_class("sel.stool.rim", "edge.stool.rim");
    let create = operation(
        "op.stool.create_seat",
        ModelingOperationKind::PrimitiveCreate,
        Vec::new(),
        vec![
            scalar_param("width", rng.scalar(0.42, 0.58)),
            scalar_param("depth", rng.scalar(0.38, 0.52)),
            scalar_param("thickness", rng.scalar(0.045, 0.08)),
        ],
        14,
    );
    let inset = operation(
        "op.stool.seat_inset",
        ModelingOperationKind::RegionInset,
        vec![seat_top.id.clone()],
        vec![scalar_param("inset", rng.scalar(0.025, 0.045))],
        10,
    );
    let back_extrude = operation(
        "op.stool.back_extrude",
        ModelingOperationKind::RegionExtrude,
        vec![back.id.clone()],
        vec![scalar_param("distance", rng.scalar(0.28, 0.42))],
        18,
    );
    let leg_array = operation(
        "op.stool.leg_array",
        ModelingOperationKind::Array,
        vec![leg.id.clone()],
        vec![
            integer_param("count", 4),
            vector_param(
                "step",
                [rng.scalar(0.18, 0.24), 0.0, rng.scalar(0.18, 0.24)],
            ),
        ],
        48,
    );
    let bevel = operation(
        "op.stool.soft_bevel",
        ModelingOperationKind::Bevel,
        vec![rim.id.clone()],
        vec![scalar_param("radius", rng.scalar(0.01, 0.025))],
        30,
    );
    let mut program = program_from_parts(
        None,
        vec![seat_top, leg, back, rim],
        vec![create, inset, back_extrude, leg_array, bevel],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);

    finish_case(
        "generated.furniture_workshop_stool",
        "Workshop stool",
        program,
        "furniture",
        scale,
        RawGeometrySize {
            vertex_count: 164,
            face_count: 118,
            position_bytes: 164 * 3 * 8,
            topology_bytes: 118 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 6,
            regions: 18,
            boundary_loops: 24,
        },
        CorpusDifficultyTier::Moderate,
        5,
        7,
        Vec::new(),
        Vec::new(),
        rng,
    )
}

fn modular_wall_segment_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let wall = selection_part("sel.wall.host", "part.wall.segment");
    let opening = selection_boolean_operand("sel.wall.opening_operand", "operand.wall.window");
    let trim = selection_edge_class("sel.wall.trim", "edge.wall.trim");
    let panel = selection_region("sel.wall.panel", "region.wall.panel");
    let create = operation(
        "op.wall.create",
        ModelingOperationKind::PrimitiveCreate,
        Vec::new(),
        vec![
            scalar_param("width", rng.scalar(1.8, 2.4)),
            scalar_param("height", rng.scalar(2.4, 3.0)),
            scalar_param("thickness", rng.scalar(0.16, 0.28)),
        ],
        14,
    );
    let panel_inset = operation(
        "op.wall.panel_inset",
        ModelingOperationKind::RegionInset,
        vec![panel.id.clone()],
        vec![scalar_param("inset", rng.scalar(0.04, 0.08))],
        16,
    );
    let window_cut = operation(
        "op.wall.window_cut",
        ModelingOperationKind::ConstrainedBoolean,
        vec![wall.id.clone(), opening.id.clone()],
        vec![
            choice_param("operation", "subtract"),
            scalar_param("clearance", rng.scalar(0.012, 0.025)),
        ],
        28,
    );
    let module_array = operation(
        "op.wall.module_array",
        ModelingOperationKind::Array,
        vec![wall.id.clone()],
        vec![
            integer_param("count", 3),
            vector_param("step", [rng.scalar(1.9, 2.5), 0.0, 0.0]),
        ],
        54,
    );
    let trim_bevel = operation(
        "op.wall.trim_bevel",
        ModelingOperationKind::Bevel,
        vec![trim.id.clone()],
        vec![scalar_param("radius", rng.scalar(0.012, 0.03))],
        32,
    );
    let mut program = program_from_parts(
        None,
        vec![wall, opening, trim, panel],
        vec![create, panel_inset, window_cut, module_array, trim_bevel],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);

    finish_case(
        "generated.modular_wall_segment",
        "Modular wall segment",
        program,
        "modular_architecture",
        scale,
        RawGeometrySize {
            vertex_count: 216,
            face_count: 160,
            position_bytes: 216 * 3 * 8,
            topology_bytes: 160 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 3,
            regions: 24,
            boundary_loops: 34,
        },
        CorpusDifficultyTier::Hard,
        6,
        10,
        Vec::new(),
        Vec::new(),
        rng,
    )
}

fn stylized_organic_cactus_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let stem = selection_region("sel.cactus.stem", "region.cactus.stem");
    let cap = selection_region("sel.cactus.cap", "region.cactus.cap");
    let rib_edges = selection_edge_class("sel.cactus.ribs", "edge.cactus.ribs");
    let create = operation(
        "op.cactus.create_stem",
        ModelingOperationKind::PrimitiveCreate,
        Vec::new(),
        vec![
            scalar_param("radius", rng.scalar(0.11, 0.18)),
            scalar_param("height", rng.scalar(0.55, 0.95)),
            integer_param("segments", 12),
        ],
        18,
    );
    let taper = operation(
        "op.cactus.taper",
        ModelingOperationKind::Taper,
        vec![stem.id.clone()],
        vec![
            vector_param("axis", [0.0, 1.0, 0.0]),
            scalar_param("amount", rng.scalar(0.08, 0.18)),
            scalar_param("falloff", rng.scalar(0.6, 0.9)),
        ],
        36,
    );
    let bend = operation(
        "op.cactus.bend",
        ModelingOperationKind::Bend,
        vec![stem.id.clone()],
        vec![
            vector_param("axis", [0.0, 0.0, 1.0]),
            scalar_param("amount", rng.scalar(0.04, 0.12)),
            scalar_param("falloff", rng.scalar(0.7, 0.95)),
        ],
        36,
    );
    let bulge = operation(
        "op.cactus.cap_bulge",
        ModelingOperationKind::Bulge,
        vec![cap.id.clone()],
        vec![
            vector_param("axis", [0.0, 1.0, 0.0]),
            scalar_param("amount", rng.scalar(0.06, 0.14)),
            scalar_param("falloff", rng.scalar(0.55, 0.85)),
        ],
        28,
    );
    let rib_bevel = operation(
        "op.cactus.rib_soften",
        ModelingOperationKind::Bevel,
        vec![rib_edges.id.clone()],
        vec![scalar_param("radius", rng.scalar(0.004, 0.012))],
        36,
    );
    let relax = operation(
        "op.cactus.smooth_relax",
        ModelingOperationKind::SmoothRelax,
        vec![stem.id.clone()],
        vec![
            integer_param("iterations", 2),
            scalar_param("strength", rng.scalar(0.1, 0.22)),
        ],
        48,
    );
    let mut program = program_from_parts(
        None,
        vec![stem, cap, rib_edges],
        vec![create, taper, bend, bulge, rib_bevel, relax],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);

    finish_case(
        "generated.stylized_organic_cactus",
        "Stylized organic cactus",
        program,
        "simple_stylized_organic_props",
        scale,
        RawGeometrySize {
            vertex_count: 192,
            face_count: 144,
            position_bytes: 192 * 3 * 8,
            topology_bytes: 144 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 1,
            regions: 16,
            boundary_loops: 18,
        },
        CorpusDifficultyTier::Moderate,
        6,
        8,
        Vec::new(),
        Vec::new(),
        rng,
    )
}

fn humanoid_blockout_case(rng: &mut SeededCorpusRng) -> GeneratedModelingCorpusCase {
    let scale = rng.scale();
    let torso = selection_part("sel.humanoid.torso", "part.humanoid.torso");
    let left_arm = selection_part("sel.humanoid.left_arm", "part.humanoid.left_arm");
    let shoulder = selection_region("sel.humanoid.shoulder", "region.humanoid.shoulder");
    let silhouette = selection_edge_class("sel.humanoid.silhouette", "edge.humanoid.silhouette");
    let arm_pose = operation(
        "op.humanoid.pose_left_arm",
        ModelingOperationKind::PartTransform,
        vec![left_arm.id.clone()],
        vec![
            vector_param(
                "translation",
                [rng.scalar(-0.04, 0.04), rng.scalar(0.02, 0.08), 0.0],
            ),
            SemanticParameter::Quaternion {
                name: "rotation".to_owned(),
                value: [0.0, 0.0, 0.0, 1.0],
            },
            vector_param("scale", [1.0, rng.scalar(0.92, 1.08), 1.0]),
        ],
        40,
    );
    let shoulder_bend = operation(
        "op.humanoid.shoulder_bend",
        ModelingOperationKind::Bend,
        vec![shoulder.id.clone()],
        vec![
            vector_param("axis", [1.0, 0.0, 0.0]),
            scalar_param("amount", rng.scalar(0.03, 0.09)),
            scalar_param("falloff", rng.scalar(0.65, 0.9)),
        ],
        42,
    );
    let mirror = operation(
        "op.humanoid.mirror_pose",
        ModelingOperationKind::Mirror,
        vec![left_arm.id.clone()],
        vec![choice_param("axis", "x"), boolean_param("weld", false)],
        52,
    );
    let joint_chain = operation(
        "op.humanoid.limb_chain",
        ModelingOperationKind::JointChainDeformation,
        vec![torso.id.clone()],
        vec![
            vector_param("root_axis", [0.0, 1.0, 0.0]),
            integer_param("joint_count", 5),
            scalar_param("pose_amount", rng.scalar(0.08, 0.18)),
            scalar_param("chain_blend", rng.scalar(0.3, 0.7)),
        ],
        64,
    );
    let silhouette_bevel = operation(
        "op.humanoid.blockout_soft_edges",
        ModelingOperationKind::Bevel,
        vec![silhouette.id.clone()],
        vec![scalar_param("radius", rng.scalar(0.01, 0.025))],
        48,
    );
    let mut program = program_from_parts(
        Some(BaseTopologyReference {
            catalog_id: "known-base-humanoid-blockout".to_owned(),
            version: "1.0".to_owned(),
            fingerprint: fingerprint("base.humanoid", rng),
        }),
        vec![torso, left_arm, shoulder, silhouette],
        vec![
            arm_pose,
            shoulder_bend,
            mirror,
            joint_chain,
            silhouette_bevel,
        ],
    );
    program.dependency_graph.operation_edges = sequential_edges(&program.operations);
    program.dependency_graph.selection_edges = selection_edges(&program.operations);

    finish_case(
        "generated.known_base_humanoid_blockout",
        "Known-base humanoid blockout",
        program,
        "known_base_humanoid_blockouts",
        scale,
        RawGeometrySize {
            vertex_count: 240,
            face_count: 176,
            position_bytes: 240 * 3 * 8,
            topology_bytes: 176 * 4 * 4,
        },
        MeshSemanticCounts {
            parts: 8,
            regions: 28,
            boundary_loops: 32,
        },
        CorpusDifficultyTier::Hard,
        7,
        9,
        Vec::new(),
        Vec::new(),
        rng,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_case(
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
    adversarial_equivalent_histories: Vec<EquivalentProgramHistory>,
    ambiguous_programs: Vec<AmbiguousProgramDescriptor>,
    rng: &mut SeededCorpusRng,
) -> GeneratedModelingCorpusCase {
    let output_fingerprint = replay_output_fingerprint(&program);
    finish_case_with_output(
        id,
        label,
        program,
        family,
        coordinate_scale,
        raw_geometry_size,
        semantic_counts,
        tier,
        expected_search_depth,
        branching_factor_hint,
        output_fingerprint,
        adversarial_equivalent_histories,
        ambiguous_programs,
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

fn selection_spatial_box(id: &str, min: [f64; 3], max: [f64; 3]) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::SpatialPrimitive {
            shape: SpatialPrimitiveSelection::Box { min, max },
        },
    }
}

fn selection_boolean_operand(id: &str, operand_id: &str) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::BooleanOperand {
            operand_id: operand_id.to_owned(),
        },
    }
}

fn selection_indices(id: &str, target: ExplicitSelectionTarget, count: usize) -> SemanticSelection {
    SemanticSelection {
        id: SemanticSelectionId(id.to_owned()),
        payload: SemanticSelectionPayload::ExplicitIndices {
            target,
            indices: (0..u32::try_from(count).expect("explicit selection count fits u32"))
                .collect(),
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

fn boolean_param(name: &str, value: bool) -> SemanticParameter {
    SemanticParameter::Boolean {
        name: name.to_owned(),
        value,
    }
}

fn choice_param(name: &str, value: &str) -> SemanticParameter {
    SemanticParameter::Choice {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

fn vector_param(name: &str, value: [f64; 3]) -> SemanticParameter {
    SemanticParameter::Vector3 {
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

fn edge(producer: &str, consumer: &str) -> (ProgramOperationId, ProgramOperationId) {
    (
        ProgramOperationId(producer.to_owned()),
        ProgramOperationId(consumer.to_owned()),
    )
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
        let bracket = corpus
            .case_by_id("generated.mirrored_array_bracket")
            .expect("bracket case should exist");

        assert_eq!(bracket.difficulty.tier, CorpusDifficultyTier::Hard);
        assert_eq!(
            bracket.difficulty.operation_count,
            bracket.program.operations.len()
        );
        assert_eq!(
            bracket.difficulty.selection_count,
            bracket.program.selections.len()
        );
        assert_eq!(
            bracket.difficulty.semantic_parameter_count,
            semantic_parameter_count(&bracket.program)
        );
        assert_eq!(bracket.difficulty.equivalent_history_count, 1);
        assert!(bracket.difficulty.expected_search_depth >= 6);
        assert!(bracket.difficulty.branching_factor_hint >= 8);
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
        let bracket = corpus
            .case_by_id("generated.mirrored_array_bracket")
            .expect("bracket case should exist");
        let keyway = corpus
            .case_by_id("generated.ambiguous_keyway")
            .expect("keyway case should exist");

        assert!(bracket.flags.adversarial_equivalent_histories);
        assert!(bracket.difficulty.has_adversarial_equivalent_histories);
        assert_eq!(bracket.adversarial_equivalent_histories.len(), 1);
        assert_eq!(
            bracket.expected_exact_output.output_fingerprint,
            replay_output_fingerprint(&bracket.program)
        );
        assert_eq!(
            bracket.adversarial_equivalent_histories[0].shared_output_fingerprint,
            bracket.expected_exact_output.output_fingerprint
        );

        assert!(keyway.flags.ambiguous_programs);
        assert!(keyway.difficulty.has_ambiguous_programs);
        assert_eq!(keyway.ambiguous_programs.len(), 1);
        assert_eq!(
            keyway.ambiguous_programs[0].expected_acceptance,
            AmbiguousProgramAcceptance::RankBelowCanonical
        );
        assert_eq!(
            keyway.expected_exact_output.output_fingerprint,
            replay_output_fingerprint(&keyway.program)
        );
        assert_eq!(
            keyway.ambiguous_programs[0].expected_output_fingerprint,
            replay_output_fingerprint(&keyway.ambiguous_programs[0].program)
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
