//! Deterministic inverse program search contracts.
//!
//! This module defines contract-level search over semantic modeling programs.
//! It deliberately does not solve geometry. Search nodes carry candidate
//! programs and exact-output metadata; the search contract ranks them
//! deterministically and accepts only zero-residual candidates whose canonical
//! semantic-output fingerprint matches the target.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_program::corpus::ExpectedExactOutputDescriptor;
use shape_program::evaluator::{EvaluatorConfig, semantic_output_fingerprint};
use shape_program::{
    ModelingOperationKind, ModelingProgram, OperationPayloadKind, RawGeometrySize,
    SemanticAdmissibilityPolicy, SemanticParameter, SemanticSelectionId, SemanticSelectionPayload,
    SemanticTopologyExact, SerializationOrderExact,
};
use shape_program_verify::{
    StrictVerificationEvidence, StrictVerificationIssue, verify_strict_semantic_program,
};

/// Stable search-node ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SearchNodeId(pub String);

/// Search strategy used to rank and trim the frontier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramSearchStrategy {
    /// Beam search with a fixed deterministic frontier width.
    Beam {
        /// Maximum number of frontier nodes retained after every expansion.
        width: usize,
    },
    /// A* style ordering using the objective plus each node's estimated
    /// remaining cost.
    AStar,
    /// Beam search whose retained width grows deterministically by depth.
    DynamicBeam {
        /// Width at depth zero.
        min_width: usize,
        /// Upper retained width.
        max_width: usize,
        /// Additional width per minimum frontier depth.
        growth_per_depth: usize,
    },
}

impl ProgramSearchStrategy {
    fn retained_width(self, frontier_min_depth: usize) -> Option<usize> {
        match self {
            Self::Beam { width } => Some(width.max(1)),
            Self::AStar => None,
            Self::DynamicBeam {
                min_width,
                max_width,
                growth_per_depth,
            } => {
                let grown = min_width
                    .max(1)
                    .saturating_add(growth_per_depth.saturating_mul(frontier_min_depth));
                Some(grown.min(max_width.max(1)))
            }
        }
    }
}

/// Weights for the inverse-search objective.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramSearchObjectiveWeights {
    /// Serialized semantic-program description length.
    pub description_length: u64,
    /// Operation, parameter, and semantic payload complexity.
    pub semantic_complexity: u64,
    /// Hypothesis instability and non-perturbable local controls.
    pub instability: u64,
    /// Cost of identifying compact and explicit semantic selections.
    pub selection_complexity: u64,
    /// A* estimated remaining cost. Ignored by pure beam ordering.
    pub estimated_remaining: u64,
}

impl Default for ProgramSearchObjectiveWeights {
    fn default() -> Self {
        Self {
            description_length: 1,
            semantic_complexity: 16,
            instability: 32,
            selection_complexity: 8,
            estimated_remaining: 1,
        }
    }
}

/// Hard limits for contract-level search.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramSearchLimits {
    /// Maximum nodes expanded.
    pub max_expanded_nodes: usize,
    /// Maximum nodes retained in the frontier.
    pub max_frontier_nodes: usize,
    /// Maximum search depth expanded.
    pub max_depth: usize,
    /// Maximum strict successes retained before stopping.
    pub max_successes: usize,
}

impl Default for ProgramSearchLimits {
    fn default() -> Self {
        Self {
            max_expanded_nodes: 512,
            max_frontier_nodes: 256,
            max_depth: 32,
            max_successes: 1,
        }
    }
}

/// Exact output requirement for strict inverse success.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExactOutputRequirement {
    /// Canonical evaluator configuration used to fingerprint each program.
    pub evaluator_config: EvaluatorConfig,
    /// Expected semantic output fingerprint.
    pub expected_output_fingerprint: String,
    /// Optional corpus descriptor for exactness channels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_descriptor: Option<ExpectedExactOutputDescriptor>,
    /// Deprecated compatibility flag. Strict success always requires zero
    /// residual bytes, even if this is false in older serialized searches.
    pub require_zero_residual: bool,
    /// Deprecated compatibility flag. Strict success always rejects literal
    /// target mesh payloads, even if this is false in older serialized searches.
    pub require_no_literal_target_mesh: bool,
    /// Deprecated compatibility flag. Strict success always rejects independent
    /// per-vertex position parameters, even if this is false in older searches.
    pub require_no_per_vertex_positions: bool,
}

impl ExactOutputRequirement {
    /// Construct a strict exact-output requirement from an expected fingerprint.
    #[must_use]
    pub fn fingerprint(expected_output_fingerprint: impl Into<String>) -> Self {
        Self {
            evaluator_config: EvaluatorConfig::canonical(),
            expected_output_fingerprint: expected_output_fingerprint.into(),
            expected_descriptor: None,
            require_zero_residual: true,
            require_no_literal_target_mesh: true,
            require_no_per_vertex_positions: true,
        }
    }

    /// Construct a strict exact-output requirement from a generated corpus
    /// descriptor.
    #[must_use]
    pub fn from_expected_descriptor(descriptor: ExpectedExactOutputDescriptor) -> Self {
        Self {
            evaluator_config: EvaluatorConfig::canonical(),
            expected_output_fingerprint: descriptor.output_fingerprint.clone(),
            expected_descriptor: Some(descriptor),
            require_zero_residual: true,
            require_no_literal_target_mesh: true,
            require_no_per_vertex_positions: true,
        }
    }
}

/// Candidate search node. The program is a full semantic candidate; the extra
/// counters are contract-level evidence about whether the candidate smuggled
/// residual or target-specific data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSearchNodeDescriptor {
    /// Stable node ID.
    pub id: SearchNodeId,
    /// Parent node, if this was generated by expanding another node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<SearchNodeId>,
    /// Search depth.
    pub depth: usize,
    /// Candidate semantic modeling program.
    pub program: ModelingProgram,
    /// Residual bytes needed by this candidate. Must be zero for strict success.
    pub residual_bytes: usize,
    /// Literal target mesh bytes embedded by this candidate.
    pub literal_target_mesh_bytes: usize,
    /// Independent per-vertex position parameters used by this candidate.
    pub per_vertex_independent_position_parameters: usize,
    /// Deterministic instability penalty emitted by hypothesis generation.
    pub hypothesis_instability: u64,
    /// Deterministic remaining-cost estimate for A* ordering.
    pub estimated_remaining_cost: u64,
    /// Generator-provided diagnostics to carry into failure reports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostic_hooks: Vec<ProgramSearchFailureHook>,
}

impl ProgramSearchNodeDescriptor {
    /// Construct a strict candidate with no residual evidence.
    #[must_use]
    pub fn strict_candidate(id: impl Into<String>, depth: usize, program: ModelingProgram) -> Self {
        Self {
            id: SearchNodeId(id.into()),
            parent: None,
            depth,
            program,
            residual_bytes: 0,
            literal_target_mesh_bytes: 0,
            per_vertex_independent_position_parameters: 0,
            hypothesis_instability: 0,
            estimated_remaining_cost: 0,
            diagnostic_hooks: Vec::new(),
        }
    }
}

/// Contract-level search graph. Children are precomputed by hypothesis
/// generation; search only ranks, expands, limits, and validates them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSearchSpace {
    /// Initial frontier.
    pub roots: Vec<ProgramSearchNodeDescriptor>,
    /// Successors keyed by parent node ID.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub successors: BTreeMap<SearchNodeId, Vec<ProgramSearchNodeDescriptor>>,
}

impl ProgramSearchSpace {
    /// Construct a search space with root nodes and no successors.
    #[must_use]
    pub fn roots(roots: Vec<ProgramSearchNodeDescriptor>) -> Self {
        Self {
            roots,
            successors: BTreeMap::new(),
        }
    }
}

/// Complete search problem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSearchProblem {
    /// Frontier ordering and trimming strategy.
    pub strategy: ProgramSearchStrategy,
    /// Objective-term weights.
    pub objective: ProgramSearchObjectiveWeights,
    /// Search hard limits.
    pub limits: ProgramSearchLimits,
    /// Exact-output requirement.
    pub exact_output: ExactOutputRequirement,
    /// Candidate graph supplied by hypothesis generation.
    pub space: ProgramSearchSpace,
}

impl ProgramSearchProblem {
    /// Run deterministic contract-level search.
    #[must_use]
    pub fn run(&self) -> ProgramSearchReport {
        search_modeling_programs(self)
    }
}

/// One score vector for a search node.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramSearchScore {
    /// Serialized semantic-program description length in bytes.
    pub description_length: u64,
    /// Semantic operation and control complexity.
    pub semantic_complexity: u64,
    /// Candidate instability penalty.
    pub instability: u64,
    /// Semantic selection complexity.
    pub selection_complexity: u64,
    /// A* estimated remaining cost supplied by hypothesis generation.
    pub estimated_remaining: u64,
    /// Weighted score without estimated remaining cost.
    pub total_without_estimate: u64,
    /// Weighted score including estimated remaining cost.
    pub total_with_estimate: u64,
}

/// Exact-output check for one node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactOutputCheck {
    /// Evaluator version requested by the candidate matches the search
    /// requirement.
    pub evaluator_version_matches: bool,
    /// Optional expected descriptor declares strict exactness.
    pub expected_descriptor_is_strict: bool,
    /// Candidate semantic output fingerprint, if canonical replay reached that
    /// point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_fingerprint: Option<String>,
    /// Candidate fingerprint equals the expected output fingerprint.
    pub fingerprint_matches: bool,
    /// Candidate carries no residual bytes.
    pub residual_free: bool,
    /// Candidate carries no literal target mesh.
    pub literal_target_free: bool,
    /// Candidate carries no independent per-vertex position controls.
    pub per_vertex_position_free: bool,
    /// True only when all strict success gates are satisfied.
    pub strict_success: bool,
    /// Deterministic rejection reasons.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejection_reasons: Vec<StrictSuccessRejection>,
}

/// Why a candidate could not count as strict inverse success.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StrictSuccessRejection {
    /// Program requested a different evaluator version.
    EvaluatorVersionMismatch {
        /// Expected evaluator version.
        expected: String,
        /// Candidate evaluator version.
        actual: String,
    },
    /// The expected descriptor itself is not strict exact output.
    ExpectedDescriptorNotStrict {
        /// Descriptor evaluator version.
        evaluator_version: String,
    },
    /// The canonical evaluator could not replay/fingerprint the candidate.
    EvaluatorRejected {
        /// Deterministic diagnostic message.
        message: String,
    },
    /// Candidate output fingerprint did not match the target.
    OutputFingerprintMismatch {
        /// Expected fingerprint.
        expected: String,
        /// Actual fingerprint, when available.
        actual: Option<String>,
    },
    /// Candidate requires residual bytes.
    ResidualBytes {
        /// Residual byte count.
        residual_bytes: usize,
    },
    /// Candidate embeds literal target-mesh bytes.
    LiteralTargetMeshBytes {
        /// Literal target-mesh byte count.
        literal_target_mesh_bytes: usize,
    },
    /// Candidate uses independent per-vertex position controls.
    PerVertexIndependentPositions {
        /// Independent per-vertex parameter count.
        parameter_count: usize,
    },
    /// The shared strict semantic verifier rejected the candidate program.
    StrictSemanticVerificationRejected {
        /// Stable issue summaries from strict verification.
        issues: Vec<String>,
    },
}

/// Search limit category.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgramSearchLimitKind {
    /// Expanded-node budget.
    ExpandedNodes,
    /// Frontier node budget.
    FrontierNodes,
    /// Strategy beam width.
    BeamWidth,
    /// Search depth.
    MaxDepth,
    /// Strict-success count.
    Successes,
}

/// Search limit event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramSearchLimitEvent {
    /// Limit kind.
    pub kind: ProgramSearchLimitKind,
    /// Search stage.
    pub stage: String,
    /// Numeric limit.
    pub limit: usize,
    /// Observed or attempted count.
    pub observed: usize,
}

/// Failure-reporting hooks emitted by deterministic search.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgramSearchFailureHook {
    /// A hard search limit was reached.
    LimitReached {
        /// Limit category.
        limit_kind: ProgramSearchLimitKind,
        /// Search stage.
        stage: String,
        /// Configured limit.
        limit: usize,
        /// Observed or attempted count.
        observed: usize,
    },
    /// A node could not be strict success.
    ExactOutputRejected {
        /// Node ID.
        node_id: SearchNodeId,
        /// Deterministic rejection reasons.
        reasons: Vec<StrictSuccessRejection>,
    },
    /// A duplicate node ID was ignored.
    DuplicateNodeSkipped {
        /// Duplicate node ID.
        node_id: SearchNodeId,
    },
    /// Generator-provided diagnostic.
    CandidateDiagnostic {
        /// Node ID.
        node_id: SearchNodeId,
        /// Stable diagnostic code.
        code: String,
        /// Human-readable message.
        message: String,
    },
}

/// Expanded node record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramSearchExpansionRecord {
    /// Node ID.
    pub node_id: SearchNodeId,
    /// Parent node ID, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<SearchNodeId>,
    /// Search depth.
    pub depth: usize,
    /// Objective score.
    pub score: ProgramSearchScore,
    /// Exact-output status.
    pub exact_output: ExactOutputCheck,
}

/// Strict search success.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSearchSuccess {
    /// Successful node ID.
    pub node_id: SearchNodeId,
    /// Accepted semantic program.
    pub program: ModelingProgram,
    /// Objective score.
    pub score: ProgramSearchScore,
    /// Exact-output check proving strict success.
    pub exact_output: ExactOutputCheck,
}

/// Full deterministic search report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgramSearchReport {
    /// Strategy used by this search.
    pub strategy: ProgramSearchStrategy,
    /// Expanded nodes in exact deterministic order.
    pub expanded_nodes: Vec<ProgramSearchExpansionRecord>,
    /// Strict successes, sorted by the same deterministic objective.
    pub successes: Vec<ProgramSearchSuccess>,
    /// Best partial node by deterministic objective when no strict success was
    /// accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_partial: Option<ProgramSearchExpansionRecord>,
    /// Limit events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limit_events: Vec<ProgramSearchLimitEvent>,
    /// Failure-reporting hooks for diagnostics and downstream failure reports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_hooks: Vec<ProgramSearchFailureHook>,
}

impl ProgramSearchReport {
    /// Return true when at least one zero-residual exact candidate was accepted.
    #[must_use]
    pub fn has_strict_success(&self) -> bool {
        !self.successes.is_empty()
    }
}

/// Run deterministic contract-level search over candidate semantic programs.
#[must_use]
pub fn search_modeling_programs(problem: &ProgramSearchProblem) -> ProgramSearchReport {
    let mut report = ProgramSearchReport {
        strategy: problem.strategy,
        expanded_nodes: Vec::new(),
        successes: Vec::new(),
        best_partial: None,
        limit_events: Vec::new(),
        failure_hooks: Vec::new(),
    };

    if problem.limits.max_successes == 0 {
        push_limit(
            &mut report,
            ProgramSearchLimitKind::Successes,
            "search.start",
            0,
            0,
        );
        return report;
    }

    let mut frontier = score_nodes(&problem.space.roots, problem);
    sort_frontier(&mut frontier, problem.strategy);
    enforce_frontier_limits(&mut frontier, problem, &mut report, "search.roots");

    let mut expanded_ids = BTreeSet::new();
    while let Some(scored) = pop_next(&mut frontier, problem.strategy) {
        if report.expanded_nodes.len() >= problem.limits.max_expanded_nodes {
            let observed = report.expanded_nodes.len().saturating_add(1);
            push_limit(
                &mut report,
                ProgramSearchLimitKind::ExpandedNodes,
                "search.expand",
                problem.limits.max_expanded_nodes,
                observed,
            );
            break;
        }

        if !expanded_ids.insert(scored.node.id.clone()) {
            report
                .failure_hooks
                .push(ProgramSearchFailureHook::DuplicateNodeSkipped {
                    node_id: scored.node.id,
                });
            continue;
        }

        report
            .failure_hooks
            .extend(scored.node.diagnostic_hooks.clone());

        let exact_output = check_exact_output(&scored.node, &problem.exact_output);
        if !exact_output.strict_success {
            report
                .failure_hooks
                .push(ProgramSearchFailureHook::ExactOutputRejected {
                    node_id: scored.node.id.clone(),
                    reasons: exact_output.rejection_reasons.clone(),
                });
        }

        let record = ProgramSearchExpansionRecord {
            node_id: scored.node.id.clone(),
            parent: scored.node.parent.clone(),
            depth: scored.node.depth,
            score: scored.score,
            exact_output: exact_output.clone(),
        };
        update_best_partial(&mut report.best_partial, record.clone(), problem.strategy);
        report.expanded_nodes.push(record);

        if exact_output.strict_success {
            report.successes.push(ProgramSearchSuccess {
                node_id: scored.node.id.clone(),
                program: scored.node.program.clone(),
                score: scored.score,
                exact_output,
            });
            sort_successes(&mut report.successes, problem.strategy);
            if report.successes.len() >= problem.limits.max_successes {
                let observed = report.successes.len();
                push_limit(
                    &mut report,
                    ProgramSearchLimitKind::Successes,
                    "search.success",
                    problem.limits.max_successes,
                    observed,
                );
                break;
            }
        }

        let children = problem
            .space
            .successors
            .get(&scored.node.id)
            .cloned()
            .unwrap_or_default();
        if scored.node.depth >= problem.limits.max_depth {
            if !children.is_empty() {
                push_limit(
                    &mut report,
                    ProgramSearchLimitKind::MaxDepth,
                    "search.expand",
                    problem.limits.max_depth,
                    scored.node.depth.saturating_add(1),
                );
            }
            continue;
        }

        frontier.extend(
            score_nodes(&children, problem)
                .into_iter()
                .filter(|child| !expanded_ids.contains(&child.node.id)),
        );
        sort_frontier(&mut frontier, problem.strategy);
        enforce_frontier_limits(&mut frontier, problem, &mut report, "search.frontier");
    }

    report
}

#[derive(Debug, Clone, PartialEq)]
struct ScoredSearchNode {
    node: ProgramSearchNodeDescriptor,
    score: ProgramSearchScore,
    tie_breaker: ProgramSearchTieBreaker,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProgramSearchTieBreaker {
    depth: usize,
    node_id: SearchNodeId,
    program_key: String,
}

fn score_nodes(
    nodes: &[ProgramSearchNodeDescriptor],
    problem: &ProgramSearchProblem,
) -> Vec<ScoredSearchNode> {
    nodes
        .iter()
        .cloned()
        .map(|node| {
            let score = score_program_node(&node, problem.objective);
            let tie_breaker = ProgramSearchTieBreaker {
                depth: node.depth,
                node_id: node.id.clone(),
                program_key: program_tie_breaker_key(&node.program),
            };
            ScoredSearchNode {
                node,
                score,
                tie_breaker,
            }
        })
        .collect()
}

fn score_program_node(
    node: &ProgramSearchNodeDescriptor,
    weights: ProgramSearchObjectiveWeights,
) -> ProgramSearchScore {
    let description_length = node
        .program
        .description_size_bytes()
        .map_or(u64::MAX / 4, |bytes| bytes as u64);
    let semantic_complexity = semantic_complexity(&node.program);
    let instability = instability(&node.program, node.hypothesis_instability);
    let selection_complexity = selection_complexity(&node.program);
    let estimated_remaining = node.estimated_remaining_cost;
    let total_without_estimate = description_length
        .saturating_mul(weights.description_length)
        .saturating_add(semantic_complexity.saturating_mul(weights.semantic_complexity))
        .saturating_add(instability.saturating_mul(weights.instability))
        .saturating_add(selection_complexity.saturating_mul(weights.selection_complexity));
    let total_with_estimate = total_without_estimate
        .saturating_add(estimated_remaining.saturating_mul(weights.estimated_remaining));

    ProgramSearchScore {
        description_length,
        semantic_complexity,
        instability,
        selection_complexity,
        estimated_remaining,
        total_without_estimate,
        total_with_estimate,
    }
}

fn semantic_complexity(program: &ModelingProgram) -> u64 {
    program
        .operations
        .iter()
        .map(|operation| {
            operation_kind_complexity(operation.kind)
                .saturating_add(parameter_complexity(&operation.parameters))
                .saturating_add(
                    operation
                        .payloads
                        .iter()
                        .map(|payload| {
                            payload.semantic_parameter_count as u64
                                + payload.encoded_bytes.div_ceil(16) as u64
                        })
                        .sum::<u64>(),
                )
        })
        .sum()
}

fn operation_kind_complexity(kind: ModelingOperationKind) -> u64 {
    match kind {
        ModelingOperationKind::PrimitiveCreate => 4,
        ModelingOperationKind::RegionExtrude
        | ModelingOperationKind::RegionInset
        | ModelingOperationKind::LoopCut
        | ModelingOperationKind::BridgeLoops
        | ModelingOperationKind::Merge
        | ModelingOperationKind::Split
        | ModelingOperationKind::Dissolve
        | ModelingOperationKind::Separate
        | ModelingOperationKind::Join => 6,
        ModelingOperationKind::Mirror | ModelingOperationKind::Array => 7,
        ModelingOperationKind::Subdivide
        | ModelingOperationKind::Bevel
        | ModelingOperationKind::ShellSolidify
        | ModelingOperationKind::ConstrainedBoolean => 9,
        ModelingOperationKind::PartTransform
        | ModelingOperationKind::RegionTransform
        | ModelingOperationKind::SurfaceSlide
        | ModelingOperationKind::ShrinkwrapProject => 10,
        ModelingOperationKind::Bend
        | ModelingOperationKind::Twist
        | ModelingOperationKind::Taper
        | ModelingOperationKind::Bulge
        | ModelingOperationKind::Lattice
        | ModelingOperationKind::Ffd
        | ModelingOperationKind::CageDeformation
        | ModelingOperationKind::JointChainDeformation
        | ModelingOperationKind::SmoothRelax => 12,
        ModelingOperationKind::BoundedCorrectiveBasis => 24,
        ModelingOperationKind::SetAllPositions
        | ModelingOperationKind::MoveVertex
        | ModelingOperationKind::DenseDisplacement
        | ModelingOperationKind::LiteralTargetMesh
        | ModelingOperationKind::OpaqueResidual
        | ModelingOperationKind::PerVertexCageWeights => 1_000,
    }
}

fn parameter_complexity(parameters: &[SemanticParameter]) -> u64 {
    parameters
        .iter()
        .map(|parameter| match parameter {
            SemanticParameter::Scalar { .. }
            | SemanticParameter::Integer { .. }
            | SemanticParameter::Boolean { .. }
            | SemanticParameter::Choice { .. } => 1,
            SemanticParameter::Vector3 { .. } => 3,
            SemanticParameter::Quaternion { .. } => 4,
        })
        .sum()
}

fn instability(program: &ModelingProgram, hypothesis_instability: u64) -> u64 {
    let non_perturbable_payloads = program
        .operations
        .iter()
        .flat_map(|operation| &operation.payloads)
        .filter(|payload| !payload.perturbation_valid)
        .count() as u64;
    let unstable_payloads = program
        .operations
        .iter()
        .flat_map(|operation| &operation.payloads)
        .filter(|payload| {
            matches!(
                payload.kind,
                OperationPayloadKind::ExplicitSelectionIndices
                    | OperationPayloadKind::DenseDisplacement
                    | OperationPayloadKind::OpaqueResidual
                    | OperationPayloadKind::LiteralTargetMesh
                    | OperationPayloadKind::PerVertexIndependentPositions
                    | OperationPayloadKind::PerVertexCageWeights
            )
        })
        .count() as u64;

    hypothesis_instability
        .saturating_add(non_perturbable_payloads.saturating_mul(25))
        .saturating_add(unstable_payloads.saturating_mul(50))
}

fn selection_complexity(program: &ModelingProgram) -> u64 {
    program
        .selections
        .iter()
        .map(|selection| match &selection.payload {
            SemanticSelectionPayload::Part { .. }
            | SemanticSelectionPayload::Region { .. }
            | SemanticSelectionPayload::BoundaryLoop { .. }
            | SemanticSelectionPayload::EdgeClass { .. }
            | SemanticSelectionPayload::FacePatch { .. }
            | SemanticSelectionPayload::BooleanOperand { .. }
            | SemanticSelectionPayload::SemanticLandmarkGroup { .. } => 1,
            SemanticSelectionPayload::SymmetryPartner { .. } => 2,
            SemanticSelectionPayload::GeodesicNeighborhood { .. } => 3,
            SemanticSelectionPayload::SpatialPrimitive { .. } => 4,
            SemanticSelectionPayload::CompactFalloffField {
                parameter_count, ..
            } => (*parameter_count as u64).max(1),
            SemanticSelectionPayload::ExplicitIndices { indices, .. } => {
                8_u64.saturating_add(indices.len() as u64)
            }
        })
        .sum()
}

fn check_exact_output(
    node: &ProgramSearchNodeDescriptor,
    requirement: &ExactOutputRequirement,
) -> ExactOutputCheck {
    let mut rejection_reasons = Vec::new();
    let expected_evaluator_version = &requirement.evaluator_config.evaluator_version;
    let evaluator_version_matches =
        node.program.canonical_evaluator_version == *expected_evaluator_version;
    if !evaluator_version_matches {
        rejection_reasons.push(StrictSuccessRejection::EvaluatorVersionMismatch {
            expected: expected_evaluator_version.clone(),
            actual: node.program.canonical_evaluator_version.clone(),
        });
    }

    let expected_descriptor_is_strict = requirement
        .expected_descriptor
        .as_ref()
        .map(ExpectedExactOutputDescriptor::is_strict_success_exact)
        .unwrap_or(true);
    if !expected_descriptor_is_strict {
        rejection_reasons.push(StrictSuccessRejection::ExpectedDescriptorNotStrict {
            evaluator_version: requirement
                .expected_descriptor
                .as_ref()
                .map(|descriptor| descriptor.canonical_evaluator_version.clone())
                .unwrap_or_default(),
        });
    }

    let output_fingerprint =
        match semantic_output_fingerprint(&node.program, &requirement.evaluator_config) {
            Ok(fingerprint) => Some(fingerprint),
            Err(error) => {
                rejection_reasons.push(StrictSuccessRejection::EvaluatorRejected {
                    message: error.to_string(),
                });
                None
            }
        };
    let fingerprint_matches =
        output_fingerprint.as_deref() == Some(requirement.expected_output_fingerprint.as_str());
    if !fingerprint_matches {
        rejection_reasons.push(StrictSuccessRejection::OutputFingerprintMismatch {
            expected: requirement.expected_output_fingerprint.clone(),
            actual: output_fingerprint.clone(),
        });
    }

    let residual_free = node.residual_bytes == 0;
    if !residual_free {
        rejection_reasons.push(StrictSuccessRejection::ResidualBytes {
            residual_bytes: node.residual_bytes,
        });
    }

    let literal_target_free = node.literal_target_mesh_bytes == 0;
    if !literal_target_free {
        rejection_reasons.push(StrictSuccessRejection::LiteralTargetMeshBytes {
            literal_target_mesh_bytes: node.literal_target_mesh_bytes,
        });
    }

    let per_vertex_position_free = node.per_vertex_independent_position_parameters == 0;
    if !per_vertex_position_free {
        rejection_reasons.push(StrictSuccessRejection::PerVertexIndependentPositions {
            parameter_count: node.per_vertex_independent_position_parameters,
        });
    }

    match verify_strict_semantic_program(
        &node.program,
        &SemanticAdmissibilityPolicy::strict(),
        verification_raw_geometry_size(node, requirement),
        &strict_verification_evidence(
            node,
            requirement,
            fingerprint_matches,
            expected_descriptor_is_strict,
        ),
    ) {
        Ok(verification) if verification.accepted => {}
        Ok(verification) => {
            rejection_reasons.push(StrictSuccessRejection::StrictSemanticVerificationRejected {
                issues: strict_verification_issue_summaries(&verification.issues),
            });
        }
        Err(error) => {
            rejection_reasons.push(StrictSuccessRejection::StrictSemanticVerificationRejected {
                issues: vec![format!("program.serialization:{error}")],
            });
        }
    }

    rejection_reasons.sort();
    rejection_reasons.dedup();
    ExactOutputCheck {
        evaluator_version_matches,
        expected_descriptor_is_strict,
        output_fingerprint,
        fingerprint_matches,
        residual_free,
        literal_target_free,
        per_vertex_position_free,
        strict_success: rejection_reasons.is_empty(),
        rejection_reasons,
    }
}

fn strict_verification_evidence(
    node: &ProgramSearchNodeDescriptor,
    requirement: &ExactOutputRequirement,
    fingerprint_matches: bool,
    expected_descriptor_is_strict: bool,
) -> StrictVerificationEvidence {
    let descriptor = requirement.expected_descriptor.as_ref();
    StrictVerificationEvidence {
        canonical_positions_exact: fingerprint_matches
            && expected_descriptor_is_strict
            && descriptor
                .map(|descriptor| descriptor.canonical_positions_exact)
                .unwrap_or(true),
        semantic_topology_exact: descriptor
            .map(|descriptor| descriptor.semantic_topology)
            .unwrap_or_else(all_semantic_topology_exact),
        serialization_order_exact: descriptor
            .map(|descriptor| descriptor.serialization_order)
            .unwrap_or_else(all_serialization_order_exact),
        residual_bytes: node.residual_bytes,
        literal_target_mesh_bytes: node.literal_target_mesh_bytes,
        per_vertex_independent_position_parameters: node.per_vertex_independent_position_parameters,
        perturbation_valid: true,
        target_index_permutation_adapter_bytes: 0,
    }
}

fn verification_raw_geometry_size(
    node: &ProgramSearchNodeDescriptor,
    requirement: &ExactOutputRequirement,
) -> RawGeometrySize {
    requirement
        .expected_descriptor
        .as_ref()
        .map(|descriptor| descriptor.raw_geometry_size)
        .unwrap_or_else(|| inferred_raw_geometry_size_for_fingerprint_only_search(&node.program))
}

fn inferred_raw_geometry_size_for_fingerprint_only_search(
    program: &ModelingProgram,
) -> RawGeometrySize {
    let minimum_bytes = program
        .description_size_bytes()
        .ok()
        .and_then(|bytes| bytes.checked_mul(3))
        .unwrap_or(usize::MAX / 4)
        .max(1);
    RawGeometrySize {
        vertex_count: 0,
        face_count: 0,
        position_bytes: minimum_bytes,
        topology_bytes: 0,
    }
}

fn all_semantic_topology_exact() -> SemanticTopologyExact {
    SemanticTopologyExact {
        graph: true,
        polygon_boundaries: true,
        winding: true,
        part_object_membership: true,
        geometry: true,
    }
}

fn all_serialization_order_exact() -> SerializationOrderExact {
    SerializationOrderExact {
        vertex_order: true,
        face_order: true,
    }
}

fn strict_verification_issue_summaries(issues: &[StrictVerificationIssue]) -> Vec<String> {
    let mut summaries = issues
        .iter()
        .map(|issue| format!("{:?}:{}:{}", issue.code, issue.path, issue.message))
        .collect::<Vec<_>>();
    summaries.sort();
    summaries.dedup();
    summaries
}

fn sort_frontier(frontier: &mut [ScoredSearchNode], strategy: ProgramSearchStrategy) {
    frontier.sort_by_key(|node| frontier_rank_key(node, strategy));
}

fn pop_next(
    frontier: &mut Vec<ScoredSearchNode>,
    strategy: ProgramSearchStrategy,
) -> Option<ScoredSearchNode> {
    sort_frontier(frontier, strategy);
    if frontier.is_empty() {
        None
    } else {
        Some(frontier.remove(0))
    }
}

fn frontier_rank_key(
    node: &ScoredSearchNode,
    strategy: ProgramSearchStrategy,
) -> ProgramSearchRankKey {
    let primary_score = match strategy {
        ProgramSearchStrategy::AStar => node.score.total_with_estimate,
        ProgramSearchStrategy::Beam { .. } | ProgramSearchStrategy::DynamicBeam { .. } => {
            node.score.total_without_estimate
        }
    };
    ProgramSearchRankKey {
        primary_score,
        description_length: node.score.description_length,
        semantic_complexity: node.score.semantic_complexity,
        instability: node.score.instability,
        selection_complexity: node.score.selection_complexity,
        estimated_remaining: node.score.estimated_remaining,
        tie_breaker: node.tie_breaker.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProgramSearchRankKey {
    primary_score: u64,
    description_length: u64,
    semantic_complexity: u64,
    instability: u64,
    selection_complexity: u64,
    estimated_remaining: u64,
    tie_breaker: ProgramSearchTieBreaker,
}

fn enforce_frontier_limits(
    frontier: &mut Vec<ScoredSearchNode>,
    problem: &ProgramSearchProblem,
    report: &mut ProgramSearchReport,
    stage: &str,
) {
    match strategy_width(frontier, problem.strategy) {
        Some(width) if frontier.len() > width => {
            let observed = frontier.len();
            frontier.truncate(width);
            push_limit(
                report,
                ProgramSearchLimitKind::BeamWidth,
                stage,
                width,
                observed,
            );
        }
        _ => {}
    }

    if frontier.len() > problem.limits.max_frontier_nodes {
        let observed = frontier.len();
        frontier.truncate(problem.limits.max_frontier_nodes);
        push_limit(
            report,
            ProgramSearchLimitKind::FrontierNodes,
            stage,
            problem.limits.max_frontier_nodes,
            observed,
        );
    }
}

fn strategy_width(frontier: &[ScoredSearchNode], strategy: ProgramSearchStrategy) -> Option<usize> {
    let frontier_min_depth = frontier
        .iter()
        .map(|node| node.node.depth)
        .min()
        .unwrap_or(0);
    strategy.retained_width(frontier_min_depth)
}

fn push_limit(
    report: &mut ProgramSearchReport,
    kind: ProgramSearchLimitKind,
    stage: &str,
    limit: usize,
    observed: usize,
) {
    let event = ProgramSearchLimitEvent {
        kind,
        stage: stage.to_owned(),
        limit,
        observed,
    };
    report.limit_events.push(event.clone());
    report
        .failure_hooks
        .push(ProgramSearchFailureHook::LimitReached {
            limit_kind: event.kind,
            stage: event.stage,
            limit: event.limit,
            observed: event.observed,
        });
}

fn update_best_partial(
    best_partial: &mut Option<ProgramSearchExpansionRecord>,
    candidate: ProgramSearchExpansionRecord,
    strategy: ProgramSearchStrategy,
) {
    let replace = best_partial
        .as_ref()
        .map(|current| {
            expansion_rank_key(&candidate, strategy) < expansion_rank_key(current, strategy)
        })
        .unwrap_or(true);
    if replace {
        *best_partial = Some(candidate);
    }
}

fn sort_successes(successes: &mut [ProgramSearchSuccess], strategy: ProgramSearchStrategy) {
    successes.sort_by_key(|success| {
        let primary_score = match strategy {
            ProgramSearchStrategy::AStar => success.score.total_with_estimate,
            ProgramSearchStrategy::Beam { .. } | ProgramSearchStrategy::DynamicBeam { .. } => {
                success.score.total_without_estimate
            }
        };
        (
            primary_score,
            success.score.description_length,
            success.score.semantic_complexity,
            success.score.instability,
            success.score.selection_complexity,
            success.score.estimated_remaining,
            success.node_id.clone(),
        )
    });
}

fn expansion_rank_key(
    expansion: &ProgramSearchExpansionRecord,
    strategy: ProgramSearchStrategy,
) -> (u64, u64, u64, u64, u64, u64, usize, SearchNodeId) {
    let primary_score = match strategy {
        ProgramSearchStrategy::AStar => expansion.score.total_with_estimate,
        ProgramSearchStrategy::Beam { .. } | ProgramSearchStrategy::DynamicBeam { .. } => {
            expansion.score.total_without_estimate
        }
    };
    (
        primary_score,
        expansion.score.description_length,
        expansion.score.semantic_complexity,
        expansion.score.instability,
        expansion.score.selection_complexity,
        expansion.score.estimated_remaining,
        expansion.depth,
        expansion.node_id.clone(),
    )
}

fn program_tie_breaker_key(program: &ModelingProgram) -> String {
    let operations = program
        .operations
        .iter()
        .map(|operation| {
            format!(
                "{}:{:?}:{}:{}",
                operation.id.0,
                operation.kind,
                stable_selection_list(&operation.selections),
                operation.parameters.len()
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    let selections = program
        .selections
        .iter()
        .map(|selection| selection.id.0.as_str())
        .collect::<Vec<_>>()
        .join("|");
    format!("{operations}::{selections}")
}

fn stable_selection_list(selections: &[SemanticSelectionId]) -> String {
    selections
        .iter()
        .map(|selection| selection.0.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_program::{
        CANONICAL_EVALUATOR_VERSION, ModelingOperation, OperationPayloadDescriptor,
        ProgramOperationId, RawGeometrySize, SemanticTopologyExact, SerializationOrderExact,
    };

    #[test]
    fn search_expansion_order_is_stable_under_ties() {
        let program = fixture_program("op.create", 0.0);
        let problem = ProgramSearchProblem {
            strategy: ProgramSearchStrategy::AStar,
            objective: ProgramSearchObjectiveWeights::default(),
            limits: ProgramSearchLimits {
                max_expanded_nodes: 8,
                max_frontier_nodes: 8,
                max_depth: 2,
                max_successes: 2,
            },
            exact_output: ExactOutputRequirement::fingerprint("not-the-target"),
            space: ProgramSearchSpace::roots(vec![
                ProgramSearchNodeDescriptor::strict_candidate("node.b", 0, program.clone()),
                ProgramSearchNodeDescriptor::strict_candidate("node.a", 0, program),
            ]),
        };

        let first = search_modeling_programs(&problem);
        let second = search_modeling_programs(&problem);

        assert_eq!(first, second);
        assert_eq!(
            expanded_ids(&first),
            vec!["node.a".to_owned(), "node.b".to_owned()]
        );
        assert!(!first.has_strict_success());
    }

    #[test]
    fn search_reports_frontier_and_expansion_limits() {
        let program = fixture_program("op.create", 0.0);
        let problem = ProgramSearchProblem {
            strategy: ProgramSearchStrategy::AStar,
            objective: ProgramSearchObjectiveWeights::default(),
            limits: ProgramSearchLimits {
                max_expanded_nodes: 1,
                max_frontier_nodes: 1,
                max_depth: 2,
                max_successes: 1,
            },
            exact_output: ExactOutputRequirement::fingerprint("not-the-target"),
            space: ProgramSearchSpace::roots(vec![
                ProgramSearchNodeDescriptor::strict_candidate("node.a", 0, program.clone()),
                ProgramSearchNodeDescriptor::strict_candidate("node.b", 0, program),
            ]),
        };

        let report = search_modeling_programs(&problem);

        assert_eq!(expanded_ids(&report), vec!["node.a".to_owned()]);
        assert!(report.limit_events.iter().any(|event| {
            event.kind == ProgramSearchLimitKind::FrontierNodes
                && event.limit == 1
                && event.observed == 2
        }));
        assert!(report.failure_hooks.iter().any(|hook| matches!(
            hook,
            ProgramSearchFailureHook::LimitReached {
                limit_kind: ProgramSearchLimitKind::FrontierNodes,
                limit: 1,
                observed: 2,
                ..
            }
        )));
    }

    #[test]
    fn zero_residual_is_required_for_strict_success() {
        let program = fixture_program("op.create", 0.25);
        let expected = semantic_output_fingerprint(&program, &EvaluatorConfig::canonical())
            .expect("fixture should fingerprint");
        let mut residual_candidate =
            ProgramSearchNodeDescriptor::strict_candidate("node.residual", 0, program.clone());
        residual_candidate.residual_bytes = 4;
        let residual_problem = ProgramSearchProblem {
            strategy: ProgramSearchStrategy::Beam { width: 4 },
            objective: ProgramSearchObjectiveWeights::default(),
            limits: ProgramSearchLimits::default(),
            exact_output: ExactOutputRequirement::fingerprint(expected.clone()),
            space: ProgramSearchSpace::roots(vec![residual_candidate]),
        };

        let residual_report = search_modeling_programs(&residual_problem);
        assert!(!residual_report.has_strict_success());
        assert!(residual_report.failure_hooks.iter().any(|hook| matches!(
            hook,
            ProgramSearchFailureHook::ExactOutputRejected { reasons, .. }
                if reasons.iter().any(|reason| matches!(
                    reason,
                    StrictSuccessRejection::ResidualBytes { residual_bytes: 4 }
                ))
        )));

        let strict_problem =
            ProgramSearchProblem {
                strategy: ProgramSearchStrategy::Beam { width: 4 },
                objective: ProgramSearchObjectiveWeights::default(),
                limits: ProgramSearchLimits::default(),
                exact_output: ExactOutputRequirement::fingerprint(expected),
                space: ProgramSearchSpace::roots(vec![
                    ProgramSearchNodeDescriptor::strict_candidate("node.strict", 0, program),
                ]),
            };

        let strict_report = search_modeling_programs(&strict_problem);
        assert!(strict_report.has_strict_success());
        assert!(strict_report.successes[0].exact_output.strict_success);
    }

    #[test]
    fn legacy_requirement_flags_cannot_relax_strict_success() {
        let program = fixture_program("op.create", 0.25);
        let expected = semantic_output_fingerprint(&program, &EvaluatorConfig::canonical())
            .expect("fixture should fingerprint");
        let mut residual_candidate =
            ProgramSearchNodeDescriptor::strict_candidate("node.residual", 0, program);
        residual_candidate.residual_bytes = 4;
        residual_candidate.literal_target_mesh_bytes = 8;
        residual_candidate.per_vertex_independent_position_parameters = 2;
        let mut exact_output = ExactOutputRequirement::fingerprint(expected);
        exact_output.require_zero_residual = false;
        exact_output.require_no_literal_target_mesh = false;
        exact_output.require_no_per_vertex_positions = false;
        let problem = ProgramSearchProblem {
            strategy: ProgramSearchStrategy::Beam { width: 4 },
            objective: ProgramSearchObjectiveWeights::default(),
            limits: ProgramSearchLimits::default(),
            exact_output,
            space: ProgramSearchSpace::roots(vec![residual_candidate]),
        };

        let report = search_modeling_programs(&problem);

        assert!(!report.has_strict_success());
        assert!(report.failure_hooks.iter().any(|hook| matches!(
            hook,
            ProgramSearchFailureHook::ExactOutputRejected { reasons, .. }
                if reasons.iter().any(|reason| matches!(
                    reason,
                    StrictSuccessRejection::ResidualBytes { residual_bytes: 4 }
                ))
                && reasons.iter().any(|reason| matches!(
                    reason,
                    StrictSuccessRejection::LiteralTargetMeshBytes {
                        literal_target_mesh_bytes: 8
                    }
                ))
                && reasons.iter().any(|reason| matches!(
                    reason,
                    StrictSuccessRejection::PerVertexIndependentPositions {
                        parameter_count: 2
                    }
                ))
        )));
    }

    #[test]
    fn forbidden_operation_cannot_be_search_success() {
        let mut program = fixture_program("op.forbidden", 0.25);
        program.operations[0].kind = ModelingOperationKind::SetAllPositions;
        let expected = semantic_output_fingerprint(&program, &EvaluatorConfig::canonical())
            .expect("fixture should fingerprint");
        let problem =
            ProgramSearchProblem {
                strategy: ProgramSearchStrategy::Beam { width: 4 },
                objective: ProgramSearchObjectiveWeights::default(),
                limits: ProgramSearchLimits::default(),
                exact_output: ExactOutputRequirement::fingerprint(expected),
                space: ProgramSearchSpace::roots(vec![
                    ProgramSearchNodeDescriptor::strict_candidate("node.forbidden", 0, program),
                ]),
            };

        let report = search_modeling_programs(&problem);

        assert!(!report.has_strict_success());
        assert!(report.failure_hooks.iter().any(|hook| matches!(
            hook,
            ProgramSearchFailureHook::ExactOutputRejected { reasons, .. }
                if reasons.iter().any(|reason| matches!(
                    reason,
                    StrictSuccessRejection::StrictSemanticVerificationRejected { issues }
                        if issues.iter().any(|issue| issue.contains("OperationNotAdmissible"))
                ))
        )));
    }

    #[test]
    fn expected_descriptor_must_itself_be_strict() {
        let program = fixture_program("op.create", 0.5);
        let expected = semantic_output_fingerprint(&program, &EvaluatorConfig::canonical())
            .expect("fixture should fingerprint");
        let mut descriptor = strict_descriptor(expected);
        descriptor.residual_bytes = 1;
        let problem =
            ProgramSearchProblem {
                strategy: ProgramSearchStrategy::AStar,
                objective: ProgramSearchObjectiveWeights::default(),
                limits: ProgramSearchLimits::default(),
                exact_output: ExactOutputRequirement::from_expected_descriptor(descriptor),
                space: ProgramSearchSpace::roots(vec![
                    ProgramSearchNodeDescriptor::strict_candidate("node.strict", 0, program),
                ]),
            };

        let report = search_modeling_programs(&problem);

        assert!(!report.has_strict_success());
        assert!(report.failure_hooks.iter().any(|hook| matches!(
            hook,
            ProgramSearchFailureHook::ExactOutputRejected { reasons, .. }
                if reasons.iter().any(|reason| matches!(
                    reason,
                    StrictSuccessRejection::ExpectedDescriptorNotStrict { .. }
                ))
        )));
    }

    fn fixture_program(operation_id: &str, offset: f64) -> ModelingProgram {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation {
            id: ProgramOperationId(operation_id.to_owned()),
            kind: ModelingOperationKind::PrimitiveCreate,
            selections: Vec::new(),
            parameters: vec![SemanticParameter::Vector3 {
                name: "center".to_owned(),
                value: [offset, 0.0, 0.0],
            }],
            affected_element_count: 16,
            payloads: vec![OperationPayloadDescriptor {
                kind: OperationPayloadKind::SemanticParameters,
                encoded_bytes: 24,
                semantic_parameter_count: 3,
                affected_element_count: 16,
                perturbation_valid: true,
            }],
        });
        program
    }

    fn strict_descriptor(output_fingerprint: String) -> ExpectedExactOutputDescriptor {
        ExpectedExactOutputDescriptor {
            canonical_evaluator_version: CANONICAL_EVALUATOR_VERSION.to_owned(),
            semantic_topology: SemanticTopologyExact {
                graph: true,
                polygon_boundaries: true,
                winding: true,
                part_object_membership: true,
                geometry: true,
            },
            serialization_order: SerializationOrderExact {
                vertex_order: true,
                face_order: true,
            },
            canonical_positions_exact: true,
            residual_bytes: 0,
            literal_target_mesh_bytes: 0,
            per_vertex_independent_position_parameters: 0,
            raw_geometry_size: RawGeometrySize {
                vertex_count: 8,
                face_count: 6,
                position_bytes: 8 * 3 * 8,
                topology_bytes: 6 * 4 * 4,
            },
            output_fingerprint,
        }
    }

    fn expanded_ids(report: &ProgramSearchReport) -> Vec<String> {
        report
            .expanded_nodes
            .iter()
            .map(|node| node.node_id.0.clone())
            .collect()
    }
}
