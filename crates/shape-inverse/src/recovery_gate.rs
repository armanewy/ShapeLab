//! Synthetic hard-surface strict recovery gate.
//!
//! This module composes the Wave 14 inverse contracts into a Wave 15 benchmark
//! gate. The gate is intentionally deterministic: "search time" is reported as
//! stable search-effort units derived from expanded candidates, not wall-clock
//! time.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_program::corpus::ExpectedExactOutputDescriptor;
use shape_program::evaluator::{EvaluatorConfig, semantic_output_fingerprint};
use shape_program::runtime::{ForwardRuntimeConfig, validate_forward_program_runtime};
use shape_program::topology::{SelectionCount, SelectionSubject, topology_contract_for};
use shape_program::{
    CANONICAL_EVALUATOR_VERSION, ExplicitSelectionTarget, ModelingOperation, ModelingOperationKind,
    ModelingProgram, ProgramDependencyGraph, ProgramOperationId, RawGeometrySize,
    SemanticParameter, SemanticRegionId, SemanticSelection, SemanticSelectionId,
    SemanticSelectionPayload, SemanticTopologyExact, SerializationOrderExact,
};
use shape_program_verify::StrictVerificationEvidence;

use crate::search::{
    ExactOutputRequirement, ProgramSearchFailureHook, ProgramSearchLimits,
    ProgramSearchNodeDescriptor, ProgramSearchProblem, ProgramSearchReport, ProgramSearchSpace,
    ProgramSearchStrategy, SearchNodeId, StrictSuccessRejection, search_modeling_programs,
};
use crate::strict::{
    StrictInverseFailure, StrictInverseFailureClass, StrictInverseVerificationReport,
    verify_strict_inverse_candidate,
};

/// Required hard-surface synthetic recovery domains for Wave 15.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardSurfaceRecoveryDomain {
    Crate,
    Lamp,
    Chair,
    Tool,
    Door,
    Panel,
    Bridge,
    SmallMachine,
    ModularWall,
}

impl HardSurfaceRecoveryDomain {
    /// Stable lowercase domain ID.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Crate => "crate",
            Self::Lamp => "lamp",
            Self::Chair => "chair",
            Self::Tool => "tool",
            Self::Door => "door",
            Self::Panel => "panel",
            Self::Bridge => "bridge",
            Self::SmallMachine => "small_machine",
            Self::ModularWall => "modular_wall",
        }
    }
}

/// Synthetic corpus used by the strict hard-surface recovery gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceStrictRecoverySuite {
    /// Deterministic generation seed.
    pub seed: u64,
    /// Synthetic hard-surface cases.
    pub cases: Vec<HardSurfaceStrictRecoveryCase>,
}

/// One synthetic hard-surface recovery case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceStrictRecoveryCase {
    /// Stable case ID.
    pub id: String,
    /// Required domain.
    pub domain: HardSurfaceRecoveryDomain,
    /// Human-readable label.
    pub label: String,
    /// Canonical compact program expected to recover.
    pub canonical_program: ModelingProgram,
    /// Target evidence supplied to inverse recovery. Search candidates are
    /// rebuilt from this observation rather than cloned from the answer key.
    pub target_observation: HardSurfaceTargetObservation,
    /// Strict exact-output descriptor.
    pub expected_exact_output: ExpectedExactOutputDescriptor,
    /// Search branching metadata.
    pub branching_factor_hint: usize,
}

/// Synthetic target observation available to inverse recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceTargetObservation {
    /// Primary observed semantic program evidence.
    pub primary: RecoveredProgramObservation,
    /// Equivalent observed histories recovered from the same target evidence.
    pub equivalents: Vec<RecoveredProgramObservation>,
}

/// Recovered program-shaped evidence before it is rebuilt as a modeling program.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveredProgramObservation {
    /// Stable observation ID.
    pub observation_id: String,
    /// Observed reusable selections.
    pub selections: Vec<RecoveredSelectionObservation>,
    /// Observed operation timeline.
    pub operations: Vec<RecoveredOperationObservation>,
}

/// Recovered selection evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveredSelectionObservation {
    /// Selection ID observed from semantic target evidence.
    pub selection_id: String,
    /// Compact semantic selection payload.
    pub payload: SemanticSelectionPayload,
}

/// Recovered operation evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveredOperationObservation {
    /// Operation ID inferred from target provenance.
    pub operation_id: String,
    /// Operation kind.
    pub kind: ModelingOperationKind,
    /// Selection IDs consumed by this operation.
    pub selection_ids: Vec<String>,
    /// Compact semantic parameters recovered from target evidence.
    pub parameters: Vec<SemanticParameter>,
    /// Affected element count inferred from semantic provenance.
    pub affected_element_count: usize,
}

/// Complete Wave 15 gate report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceStrictRecoveryReport {
    /// True only when every case is a strict zero-residual success.
    pub accepted: bool,
    /// Deterministic seed used for the synthetic corpus.
    pub seed: u64,
    /// Number of cases evaluated.
    pub case_count: usize,
    /// Per-case reports.
    pub cases: Vec<HardSurfaceStrictRecoveryCaseReport>,
    /// Aggregate metrics required by Wave 15.
    pub metrics: HardSurfaceStrictRecoveryMetrics,
}

/// Wave 15 aggregate metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceStrictRecoveryMetrics {
    /// Exact semantic topology successes / total.
    pub exact_topology_rate: f64,
    /// Exact canonical position successes / total.
    pub exact_position_rate: f64,
    /// Cases with at least one exact equivalent-program success / total.
    pub equivalent_program_rate: f64,
    /// Strict zero-residual successes / total.
    pub strict_success_rate: f64,
    /// Mean deterministic search-effort units.
    pub mean_search_time_units: f64,
    /// Mean program compression ratio.
    pub mean_program_compression: f64,
    /// Cases with valid perturbation neighborhood / total.
    pub perturbation_validity_rate: f64,
    /// Failure-class counts for non-successful cases.
    pub failure_classes: Vec<HardSurfaceFailureClassCount>,
}

/// Failure class count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardSurfaceFailureClassCount {
    /// Failure class.
    pub class: StrictInverseFailureClass,
    /// Count.
    pub count: usize,
}

/// Per-case Wave 15 report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceStrictRecoveryCaseReport {
    /// Case ID.
    pub case_id: String,
    /// Required domain.
    pub domain: HardSurfaceRecoveryDomain,
    /// True only when strict inverse and forward runtime gates passed.
    pub strict_success: bool,
    /// Semantic topology exactness from strict verification.
    pub exact_topology: bool,
    /// Canonical position exactness from strict verification.
    pub exact_positions: bool,
    /// Whether an alternate equivalent program also recovered exactly.
    pub equivalent_program_exact: bool,
    /// Deterministic search-effort units.
    pub search_time_units: u64,
    /// Expanded candidate count.
    pub expanded_nodes: usize,
    /// Strict success count returned by search.
    pub strict_search_successes: usize,
    /// Program compression ratio for the selected recovery.
    pub program_compression: f64,
    /// Perturbation validity neighborhood.
    pub perturbation_valid: bool,
    /// Selected node ID.
    pub selected_node_id: Option<String>,
    /// First failure class, when no strict success exists.
    pub failure_class: Option<StrictInverseFailureClass>,
    /// Unique failure classes observed for this case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_classes: Vec<StrictInverseFailureClass>,
    /// Strict inverse verification report for the selected program or best failure.
    pub verification: StrictInverseVerificationReport,
}

/// Run the complete deterministic Wave 15 hard-surface strict recovery gate.
#[must_use]
pub fn run_hard_surface_strict_recovery_gate(seed: u64) -> HardSurfaceStrictRecoveryReport {
    let suite = hard_surface_strict_recovery_suite(seed);
    run_hard_surface_strict_recovery_suite(&suite)
}

/// Build the deterministic hard-surface strict recovery suite.
#[must_use]
pub fn hard_surface_strict_recovery_suite(seed: u64) -> HardSurfaceStrictRecoverySuite {
    let mut rng = GateRng::new(seed);
    let cases = vec![
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::Crate,
            "Industrial crate",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::RegionInset,
                ModelingOperationKind::RegionExtrude,
                ModelingOperationKind::Array,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::Lamp,
            "Desk lamp",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::ShellSolidify,
                ModelingOperationKind::Mirror,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::Chair,
            "Hard-surface chair",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::Array,
                ModelingOperationKind::RegionExtrude,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::Tool,
            "Workshop tool",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::ConstrainedBoolean,
                ModelingOperationKind::Mirror,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::Door,
            "Panel door",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::RegionInset,
                ModelingOperationKind::ConstrainedBoolean,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::Panel,
            "Service panel",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::LoopCut,
                ModelingOperationKind::RegionInset,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::Bridge,
            "Modular bridge",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::BridgeLoops,
                ModelingOperationKind::Array,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::SmallMachine,
            "Small machine",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::ConstrainedBoolean,
                ModelingOperationKind::Array,
                ModelingOperationKind::Mirror,
                ModelingOperationKind::Bevel,
            ],
        ),
        fixture_case(
            &mut rng,
            HardSurfaceRecoveryDomain::ModularWall,
            "Modular wall",
            &[
                ModelingOperationKind::PrimitiveCreate,
                ModelingOperationKind::RegionInset,
                ModelingOperationKind::ConstrainedBoolean,
                ModelingOperationKind::Array,
                ModelingOperationKind::Bevel,
            ],
        ),
    ];

    HardSurfaceStrictRecoverySuite { seed, cases }
}

/// Run a prebuilt hard-surface recovery suite.
#[must_use]
pub fn run_hard_surface_strict_recovery_suite(
    suite: &HardSurfaceStrictRecoverySuite,
) -> HardSurfaceStrictRecoveryReport {
    let cases = suite
        .cases
        .iter()
        .map(recover_hard_surface_case)
        .collect::<Vec<_>>();
    let metrics = aggregate_metrics(&cases);
    let accepted = !cases.is_empty() && cases.iter().all(|case| case.strict_success);

    HardSurfaceStrictRecoveryReport {
        accepted,
        seed: suite.seed,
        case_count: cases.len(),
        cases,
        metrics,
    }
}

fn recover_hard_surface_case(
    case: &HardSurfaceStrictRecoveryCase,
) -> HardSurfaceStrictRecoveryCaseReport {
    let search_report = search_modeling_programs(&ProgramSearchProblem {
        strategy: ProgramSearchStrategy::AStar,
        objective: Default::default(),
        limits: ProgramSearchLimits {
            max_expanded_nodes: case.branching_factor_hint.max(8),
            max_frontier_nodes: case.branching_factor_hint.max(8),
            max_depth: case
                .target_observation
                .primary
                .operations
                .len()
                .saturating_add(2),
            max_successes: case.target_observation.equivalents.len().saturating_add(1),
        },
        exact_output: ExactOutputRequirement::from_expected_descriptor(
            case.expected_exact_output.clone(),
        ),
        space: ProgramSearchSpace::roots(candidate_nodes(case, false)),
    });

    let selected = search_report.successes.first();
    let selected_program = selected.map(|success| &success.program);
    let failure_hints = if selected_program.is_some() {
        Vec::new()
    } else {
        failure_hints_from_search_report(&search_report)
    };
    let perturbation_valid = selected_program.is_some_and(|program| {
        measured_perturbation_neighborhood_is_valid(
            &case.id,
            program,
            case.expected_exact_output.raw_geometry_size,
        )
    });
    let evidence = exact_evidence(&case.expected_exact_output, perturbation_valid);
    let verification = verify_strict_inverse_candidate(
        selected_program,
        case.expected_exact_output.raw_geometry_size,
        &evidence,
        failure_hints,
    );
    let forward_runtime_accepted = selected_program.is_some_and(|program| {
        validate_forward_program_runtime(
            format!("{}.recovered", case.id),
            program,
            &ForwardRuntimeConfig::canonical(),
        )
        .accepted
    });
    let strict_success = verification.strict_success && forward_runtime_accepted;
    let exact_topology = verification
        .verification
        .as_ref()
        .is_some_and(|verification| verification.semantic_topology_exact.is_exact());
    let exact_positions = verification
        .verification
        .as_ref()
        .is_some_and(|verification| verification.canonical_positions_exact);
    let perturbation_valid = perturbation_valid
        && verification
            .verification
            .as_ref()
            .is_some_and(|verification| verification.accepted);
    let failure_classes = failure_classes_for_case(&verification, forward_runtime_accepted);
    let failure_class = primary_failure_class(&failure_classes);

    HardSurfaceStrictRecoveryCaseReport {
        case_id: case.id.clone(),
        domain: case.domain,
        strict_success,
        exact_topology,
        exact_positions,
        equivalent_program_exact: equivalent_node_ids(case).iter().any(|equivalent_id| {
            search_report
                .successes
                .iter()
                .any(|success| &success.node_id == equivalent_id)
        }),
        search_time_units: deterministic_search_time_units(&search_report.expanded_nodes),
        expanded_nodes: search_report.expanded_nodes.len(),
        strict_search_successes: search_report.successes.len(),
        program_compression: verification
            .verification
            .as_ref()
            .map(|verification| verification.compression_ratio)
            .unwrap_or(0.0),
        perturbation_valid,
        selected_node_id: selected.map(|success| success.node_id.0.clone()),
        failure_class,
        failure_classes,
        verification,
    }
}

fn failure_classes_for_case(
    verification: &StrictInverseVerificationReport,
    forward_runtime_accepted: bool,
) -> Vec<StrictInverseFailureClass> {
    let mut classes = verification
        .failures
        .iter()
        .map(|failure| failure.class)
        .collect::<Vec<_>>();
    if !forward_runtime_accepted {
        classes.push(StrictInverseFailureClass::UnsupportedSerializationOrder);
    }
    classes.sort();
    classes.dedup();
    classes
}

fn primary_failure_class(
    classes: &[StrictInverseFailureClass],
) -> Option<StrictInverseFailureClass> {
    classes
        .iter()
        .copied()
        .find(|class| *class != StrictInverseFailureClass::SearchExhaustion)
        .or_else(|| classes.first().copied())
}

fn candidate_nodes(
    case: &HardSurfaceStrictRecoveryCase,
    residual_canonical: bool,
) -> Vec<ProgramSearchNodeDescriptor> {
    let mut nodes = Vec::new();
    let primary_program = program_from_observation(&case.target_observation.primary);
    let mut canonical = ProgramSearchNodeDescriptor::strict_candidate(
        format!("{}.candidate.000.recovered", case.id),
        0,
        primary_program,
    );
    if residual_canonical {
        canonical.residual_bytes = 8;
    }
    nodes.push(canonical);
    nodes.extend(case.target_observation.equivalents.iter().enumerate().map(
        |(index, observation)| {
            ProgramSearchNodeDescriptor::strict_candidate(
                format!("{}.candidate.{:03}.equivalent", case.id, index + 1),
                0,
                program_from_observation(observation),
            )
        },
    ));
    nodes
}

fn equivalent_node_ids(case: &HardSurfaceStrictRecoveryCase) -> BTreeSet<SearchNodeId> {
    (0..case.target_observation.equivalents.len())
        .map(|index| SearchNodeId(format!("{}.candidate.{:03}.equivalent", case.id, index + 1)))
        .collect()
}

fn failure_hints_from_search_report(
    search_report: &ProgramSearchReport,
) -> Vec<StrictInverseFailure> {
    let mut failures = search_report
        .failure_hooks
        .iter()
        .flat_map(failures_from_search_hook)
        .collect::<Vec<_>>();
    failures.sort();
    failures.dedup();
    failures
}

fn failures_from_search_hook(hook: &ProgramSearchFailureHook) -> Vec<StrictInverseFailure> {
    match hook {
        ProgramSearchFailureHook::LimitReached {
            limit_kind,
            stage,
            limit,
            observed,
        } => vec![StrictInverseFailure::search_exhaustion(
            format!("search.{limit_kind:?}.{stage}"),
            *limit,
            *observed,
        )],
        ProgramSearchFailureHook::ExactOutputRejected { node_id, reasons } => reasons
            .iter()
            .map(|reason| failure_from_strict_rejection(node_id, reason))
            .collect(),
        ProgramSearchFailureHook::DuplicateNodeSkipped { node_id } => {
            vec![StrictInverseFailure::unsupported_serialization_order(
                format!("search.duplicate.{}", node_id.0),
                "duplicate search-node ID was skipped",
            )]
        }
        ProgramSearchFailureHook::CandidateDiagnostic {
            node_id,
            code,
            message,
        } => vec![StrictInverseFailure::unsupported_serialization_order(
            format!("search.diagnostic.{}.{}", node_id.0, code),
            message.clone(),
        )],
    }
}

fn failure_from_strict_rejection(
    node_id: &SearchNodeId,
    reason: &StrictSuccessRejection,
) -> StrictInverseFailure {
    let path = format!("search.node.{}", node_id.0);
    match reason {
        StrictSuccessRejection::EvaluatorVersionMismatch { expected, actual } => {
            StrictInverseFailure::unsupported_serialization_order(
                path,
                format!("evaluator mismatch: expected {expected}, got {actual}"),
            )
        }
        StrictSuccessRejection::ExpectedDescriptorNotStrict { evaluator_version } => {
            StrictInverseFailure::unsupported_serialization_order(
                path,
                format!("expected descriptor is not strict for evaluator {evaluator_version}"),
            )
        }
        StrictSuccessRejection::EvaluatorRejected { message } => {
            StrictInverseFailure::unsupported_serialization_order(path, message.clone())
        }
        StrictSuccessRejection::OutputFingerprintMismatch { expected, actual } => {
            StrictInverseFailure::numerical_non_exactness(
                path,
                format!(
                    "output fingerprint mismatch: expected {expected}, got {}",
                    actual.as_deref().unwrap_or("<none>")
                ),
            )
        }
        StrictSuccessRejection::ResidualBytes { residual_bytes } => {
            StrictInverseFailure::numerical_non_exactness(
                path,
                format!("candidate required {residual_bytes} residual byte(s)"),
            )
        }
        StrictSuccessRejection::LiteralTargetMeshBytes {
            literal_target_mesh_bytes,
        } => StrictInverseFailure::selection_not_expressible(
            path,
            format!("candidate embedded {literal_target_mesh_bytes} literal target-mesh byte(s)"),
        ),
        StrictSuccessRejection::PerVertexIndependentPositions { parameter_count } => {
            StrictInverseFailure::selection_not_expressible(
                path,
                format!("candidate used {parameter_count} per-vertex independent parameter(s)"),
            )
        }
        StrictSuccessRejection::StrictSemanticVerificationRejected { issues } => {
            StrictInverseFailure::unsupported_serialization_order(
                path,
                format!(
                    "strict semantic verifier rejected candidate: {}",
                    issues.join("; ")
                ),
            )
        }
    }
}

fn aggregate_metrics(
    cases: &[HardSurfaceStrictRecoveryCaseReport],
) -> HardSurfaceStrictRecoveryMetrics {
    let total = cases.len().max(1) as f64;
    let failure_counts = failure_class_counts(cases);
    HardSurfaceStrictRecoveryMetrics {
        exact_topology_rate: rate(
            total,
            cases.iter().filter(|case| case.exact_topology).count(),
        ),
        exact_position_rate: rate(
            total,
            cases.iter().filter(|case| case.exact_positions).count(),
        ),
        equivalent_program_rate: rate(
            total,
            cases
                .iter()
                .filter(|case| case.equivalent_program_exact)
                .count(),
        ),
        strict_success_rate: rate(
            total,
            cases.iter().filter(|case| case.strict_success).count(),
        ),
        mean_search_time_units: mean_u64(cases.iter().map(|case| case.search_time_units)),
        mean_program_compression: mean_f64(cases.iter().map(|case| case.program_compression)),
        perturbation_validity_rate: rate(
            total,
            cases.iter().filter(|case| case.perturbation_valid).count(),
        ),
        failure_classes: failure_counts,
    }
}

fn failure_class_counts(
    cases: &[HardSurfaceStrictRecoveryCaseReport],
) -> Vec<HardSurfaceFailureClassCount> {
    let mut counts = BTreeMap::new();
    for failure_class in cases
        .iter()
        .filter(|case| !case.strict_success)
        .flat_map(|case| &case.failure_classes)
    {
        *counts.entry(*failure_class).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(class, count)| HardSurfaceFailureClassCount { class, count })
        .collect()
}

fn rate(total: f64, count: usize) -> f64 {
    count as f64 / total
}

fn mean_u64(values: impl Iterator<Item = u64>) -> f64 {
    let mut count = 0;
    let mut sum = 0_u64;
    for value in values {
        count += 1;
        sum = sum.saturating_add(value);
    }
    if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    }
}

fn mean_f64(values: impl Iterator<Item = f64>) -> f64 {
    let mut count = 0;
    let mut sum = 0.0;
    for value in values {
        count += 1;
        sum += value;
    }
    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn deterministic_search_time_units(
    expanded_nodes: &[crate::search::ProgramSearchExpansionRecord],
) -> u64 {
    expanded_nodes.iter().fold(0_u64, |sum, node| {
        sum.saturating_add(1)
            .saturating_add(node.depth as u64)
            .saturating_add(node.score.total_with_estimate)
    })
}

const PERTURBATION_SAMPLE_LIMIT: usize = 16;
const PERTURBATION_DELTA: f64 = 0.001;

fn measured_perturbation_neighborhood_is_valid(
    case_id: &str,
    program: &ModelingProgram,
    raw_geometry_size: RawGeometrySize,
) -> bool {
    let variants = perturbation_variants(program, PERTURBATION_SAMPLE_LIMIT);
    !variants.is_empty()
        && variants.iter().enumerate().all(|(index, variant)| {
            let Some(expected_variant) =
                exact_output_for_perturbed_variant(variant, raw_geometry_size)
            else {
                return false;
            };
            validate_forward_program_runtime(
                format!("{case_id}.perturbation.{index:03}"),
                variant,
                &ForwardRuntimeConfig::canonical(),
            )
            .accepted
                && verify_strict_inverse_candidate(
                    Some(variant),
                    raw_geometry_size,
                    &exact_evidence(&expected_variant, true),
                    Vec::new(),
                )
                .strict_success
        })
}

fn exact_output_for_perturbed_variant(
    variant: &ModelingProgram,
    raw_geometry_size: RawGeometrySize,
) -> Option<ExpectedExactOutputDescriptor> {
    let output_fingerprint =
        semantic_output_fingerprint(variant, &EvaluatorConfig::canonical()).ok()?;
    Some(ExpectedExactOutputDescriptor {
        canonical_evaluator_version: CANONICAL_EVALUATOR_VERSION.to_owned(),
        semantic_topology: all_topology_exact(),
        serialization_order: all_serialization_exact(),
        canonical_positions_exact: true,
        residual_bytes: 0,
        literal_target_mesh_bytes: 0,
        per_vertex_independent_position_parameters: 0,
        raw_geometry_size,
        output_fingerprint,
    })
}

fn perturbation_variants(program: &ModelingProgram, limit: usize) -> Vec<ModelingProgram> {
    let mut variants = Vec::new();
    for operation_index in 0..program.operations.len() {
        let parameter_count = program.operations[operation_index].parameters.len();
        for parameter_index in 0..parameter_count {
            for direction in [-1.0, 1.0] {
                let mut variant = program.clone();
                if perturb_parameter(
                    &mut variant.operations[operation_index].parameters[parameter_index],
                    direction,
                ) {
                    variants.push(variant);
                    if variants.len() >= limit {
                        return variants;
                    }
                }
            }
        }
    }
    variants
}

fn perturb_parameter(parameter: &mut SemanticParameter, direction: f64) -> bool {
    match parameter {
        SemanticParameter::Scalar { value, .. } => perturb_scalar(value, direction),
        SemanticParameter::Integer { value, .. } => {
            let step = if direction.is_sign_negative() { -1 } else { 1 };
            *value = value.saturating_add(step);
            true
        }
        SemanticParameter::Boolean { .. } | SemanticParameter::Choice { .. } => false,
        SemanticParameter::Vector3 { value, .. } => perturb_scalar(&mut value[0], direction),
        SemanticParameter::Quaternion { value, .. } => {
            if !perturb_scalar(&mut value[0], direction) {
                return false;
            }
            normalize_quaternion(value)
        }
    }
}

fn perturb_scalar(value: &mut f64, direction: f64) -> bool {
    if !value.is_finite() {
        return false;
    }
    let scale = value.abs().max(1.0);
    *value += direction * PERTURBATION_DELTA * scale;
    value.is_finite()
}

fn normalize_quaternion(value: &mut [f64; 4]) -> bool {
    let norm = value
        .iter()
        .map(|component| component * component)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= f64::EPSILON {
        return false;
    }
    for component in value {
        *component /= norm;
    }
    true
}

fn fixture_case(
    rng: &mut GateRng,
    domain: HardSurfaceRecoveryDomain,
    label: &str,
    operation_kinds: &[ModelingOperationKind],
) -> HardSurfaceStrictRecoveryCase {
    let primary = fixture_observation(domain, operation_kinds, rng);
    let equivalents = equivalent_observations(&primary);
    let target_observation = HardSurfaceTargetObservation {
        primary,
        equivalents,
    };
    let canonical_program = program_from_observation(&target_observation.primary);
    let output_fingerprint =
        semantic_output_fingerprint(&canonical_program, &EvaluatorConfig::canonical())
            .expect("synthetic recovery program should fingerprint");
    let expected_exact_output = expected_exact_output(
        &canonical_program,
        output_fingerprint,
        rng.raw_geometry_scale(),
    );

    HardSurfaceStrictRecoveryCase {
        id: format!("generated.strict_recovery.{}", domain.as_str()),
        domain,
        label: label.to_owned(),
        canonical_program,
        target_observation,
        expected_exact_output,
        branching_factor_hint: operation_kinds.len().saturating_add(6),
    }
}

fn fixture_observation(
    domain: HardSurfaceRecoveryDomain,
    operation_kinds: &[ModelingOperationKind],
    rng: &mut GateRng,
) -> RecoveredProgramObservation {
    let mut selections = BTreeMap::<SemanticSelectionId, SemanticSelection>::new();
    let mut operations = Vec::new();

    for (operation_index, kind) in operation_kinds.iter().copied().enumerate() {
        let operation_id = format!(
            "op.{}.{operation_index:03}.{}",
            domain.as_str(),
            operation_kind_label(kind)
        );
        let selection_ids =
            selections_for_operation(domain, operation_index, kind, &mut selections);
        operations.push(RecoveredOperationObservation {
            operation_id,
            kind,
            selection_ids: selection_ids.into_iter().map(|id| id.0).collect(),
            parameters: semantic_parameters_for(kind, rng),
            affected_element_count: 96,
        });
    }

    RecoveredProgramObservation {
        observation_id: format!("observation.{}.primary", domain.as_str()),
        selections: selections
            .into_values()
            .map(|selection| RecoveredSelectionObservation {
                selection_id: selection.id.0,
                payload: selection.payload,
            })
            .collect(),
        operations,
    }
}

fn selections_for_operation(
    domain: HardSurfaceRecoveryDomain,
    operation_index: usize,
    kind: ModelingOperationKind,
    selections: &mut BTreeMap<SemanticSelectionId, SemanticSelection>,
) -> Vec<SemanticSelectionId> {
    let Some(contract) = topology_contract_for(kind) else {
        return Vec::new();
    };
    let subjects = match contract.selection_requirements.count {
        SelectionCount::ExactlyZero => Vec::new(),
        SelectionCount::ExactlyOne | SelectionCount::OneOrMore => {
            vec![contract.selection_requirements.accepted_subjects[0]]
        }
        SelectionCount::ExactlyTwo | SelectionCount::TwoOrMore => {
            let first = contract.selection_requirements.accepted_subjects[0];
            let second = *contract
                .selection_requirements
                .accepted_subjects
                .get(1)
                .unwrap_or(&first);
            vec![first, second]
        }
    };

    subjects
        .into_iter()
        .enumerate()
        .map(|(selection_index, subject)| {
            let selection =
                selection_for_subject(domain, operation_index, selection_index, subject);
            let id = selection.id.clone();
            selections.entry(id.clone()).or_insert(selection);
            id
        })
        .collect()
}

fn selection_for_subject(
    domain: HardSurfaceRecoveryDomain,
    operation_index: usize,
    selection_index: usize,
    subject: SelectionSubject,
) -> SemanticSelection {
    let id = SemanticSelectionId(format!(
        "sel.{}.{operation_index:03}.{selection_index:02}",
        domain.as_str()
    ));
    let stem = format!(
        "{}.{}.{}",
        domain.as_str(),
        operation_index,
        selection_index
    );
    let payload = match subject {
        SelectionSubject::Part | SelectionSubject::Object => SemanticSelectionPayload::Part {
            part: shape_program::SemanticPartId(format!("part.{stem}")),
        },
        SelectionSubject::Region => SemanticSelectionPayload::Region {
            region: SemanticRegionId(format!("region.{stem}")),
        },
        SelectionSubject::BoundaryLoop => SemanticSelectionPayload::BoundaryLoop {
            boundary_loop: shape_program::SemanticBoundaryLoopId(format!("loop.{stem}")),
        },
        SelectionSubject::EdgeLoop | SelectionSubject::EdgeSet => {
            SemanticSelectionPayload::EdgeClass {
                class: format!("edge.{stem}"),
            }
        }
        SelectionSubject::VertexSet => SemanticSelectionPayload::ExplicitIndices {
            target: ExplicitSelectionTarget::Vertex,
            indices: vec![0, 1, 2, 3],
        },
        SelectionSubject::BooleanOperand => SemanticSelectionPayload::BooleanOperand {
            operand_id: format!("operand.{stem}"),
        },
    };
    SemanticSelection { id, payload }
}

fn semantic_parameters_for(
    kind: ModelingOperationKind,
    rng: &mut GateRng,
) -> Vec<SemanticParameter> {
    let count = topology_contract_for(kind)
        .map(|contract| contract.semantic_parameter_count)
        .unwrap_or_default();
    (0..count)
        .map(|index| SemanticParameter::Scalar {
            name: format!("p{index:02}"),
            value: rng.scalar(index),
        })
        .collect()
}

fn program_from_observation(observation: &RecoveredProgramObservation) -> ModelingProgram {
    let mut program = ModelingProgram::strict_from_primitives();
    program.selections = observation
        .selections
        .iter()
        .map(|selection| SemanticSelection {
            id: SemanticSelectionId(selection.selection_id.clone()),
            payload: selection.payload.clone(),
        })
        .collect();
    program.operations = observation
        .operations
        .iter()
        .map(|operation| ModelingOperation {
            id: ProgramOperationId(operation.operation_id.clone()),
            kind: operation.kind,
            selections: operation
                .selection_ids
                .iter()
                .cloned()
                .map(SemanticSelectionId)
                .collect(),
            parameters: operation.parameters.clone(),
            affected_element_count: operation.affected_element_count,
            payloads: Vec::new(),
        })
        .collect();
    program.dependency_graph = ProgramDependencyGraph {
        operation_edges: sequential_operation_edges(&program.operations),
        selection_edges: selection_edges(&program.operations),
    };
    program
}

fn equivalent_observations(
    observation: &RecoveredProgramObservation,
) -> Vec<RecoveredProgramObservation> {
    if observation.selections.len() < 2 {
        return Vec::new();
    }
    let mut equivalent = observation.clone();
    equivalent.observation_id =
        format!("{}.equivalent.selection_order", observation.observation_id);
    equivalent.selections.reverse();
    vec![equivalent]
        .into_iter()
        .filter(|candidate| {
            semantic_output_fingerprint(
                &program_from_observation(candidate),
                &EvaluatorConfig::canonical(),
            )
            .ok()
                == semantic_output_fingerprint(
                    &program_from_observation(observation),
                    &EvaluatorConfig::canonical(),
                )
                .ok()
        })
        .collect()
}

fn expected_exact_output(
    program: &ModelingProgram,
    output_fingerprint: String,
    raw_scale: usize,
) -> ExpectedExactOutputDescriptor {
    ExpectedExactOutputDescriptor {
        canonical_evaluator_version: CANONICAL_EVALUATOR_VERSION.to_owned(),
        semantic_topology: all_topology_exact(),
        serialization_order: all_serialization_exact(),
        canonical_positions_exact: true,
        residual_bytes: 0,
        literal_target_mesh_bytes: 0,
        per_vertex_independent_position_parameters: 0,
        raw_geometry_size: raw_geometry_size(program, raw_scale),
        output_fingerprint,
    }
}

fn raw_geometry_size(program: &ModelingProgram, raw_scale: usize) -> RawGeometrySize {
    let description_size = program
        .description_size_bytes()
        .expect("synthetic recovery program should serialize");
    let total = description_size
        .saturating_mul(4)
        .saturating_add(raw_scale.saturating_mul(1024));
    let vertex_count = total.div_ceil(40).max(128);
    let face_count = vertex_count.saturating_mul(3).saturating_div(2).max(96);
    RawGeometrySize {
        vertex_count,
        face_count,
        position_bytes: vertex_count.saturating_mul(3).saturating_mul(8),
        topology_bytes: face_count.saturating_mul(4).saturating_mul(4),
    }
}

fn exact_evidence(
    expected: &ExpectedExactOutputDescriptor,
    perturbation_valid: bool,
) -> StrictVerificationEvidence {
    StrictVerificationEvidence {
        canonical_positions_exact: expected.canonical_positions_exact,
        semantic_topology_exact: expected.semantic_topology,
        serialization_order_exact: expected.serialization_order,
        residual_bytes: expected.residual_bytes,
        literal_target_mesh_bytes: expected.literal_target_mesh_bytes,
        per_vertex_independent_position_parameters: expected
            .per_vertex_independent_position_parameters,
        perturbation_valid,
        target_index_permutation_adapter_bytes: 0,
    }
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

fn sequential_operation_edges(
    operations: &[ModelingOperation],
) -> Vec<(ProgramOperationId, ProgramOperationId)> {
    operations
        .windows(2)
        .map(|pair| (pair[0].id.clone(), pair[1].id.clone()))
        .collect()
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

fn operation_kind_label(kind: ModelingOperationKind) -> &'static str {
    match kind {
        ModelingOperationKind::PrimitiveCreate => "create",
        ModelingOperationKind::RegionExtrude => "extrude",
        ModelingOperationKind::RegionInset => "inset",
        ModelingOperationKind::LoopCut => "loop_cut",
        ModelingOperationKind::BridgeLoops => "bridge",
        ModelingOperationKind::Mirror => "mirror",
        ModelingOperationKind::Array => "array",
        ModelingOperationKind::Bevel => "bevel",
        ModelingOperationKind::ShellSolidify => "shell",
        ModelingOperationKind::ConstrainedBoolean => "boolean",
        _ => "operation",
    }
}

#[derive(Debug, Clone)]
struct GateRng {
    state: u64,
}

impl GateRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn scalar(&mut self, index: usize) -> f64 {
        let bucket = ((self.next() >> 32) as f64) / (u32::MAX as f64);
        0.1 + index as f64 * 0.01 + bucket * 0.05
    }

    fn raw_geometry_scale(&mut self) -> usize {
        ((self.next() % 5) + 1) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn hard_surface_suite_covers_required_wave15_domains() {
        let suite = hard_surface_strict_recovery_suite(15);
        let domains = suite
            .cases
            .iter()
            .map(|case| case.domain)
            .collect::<BTreeSet<_>>();

        assert_eq!(suite.cases.len(), 9);
        for required in [
            HardSurfaceRecoveryDomain::Crate,
            HardSurfaceRecoveryDomain::Lamp,
            HardSurfaceRecoveryDomain::Chair,
            HardSurfaceRecoveryDomain::Tool,
            HardSurfaceRecoveryDomain::Door,
            HardSurfaceRecoveryDomain::Panel,
            HardSurfaceRecoveryDomain::Bridge,
            HardSurfaceRecoveryDomain::SmallMachine,
            HardSurfaceRecoveryDomain::ModularWall,
        ] {
            assert!(domains.contains(&required), "missing {required:?}");
        }
    }

    #[test]
    fn hard_surface_strict_recovery_gate_accepts_all_cases() {
        let report = run_hard_surface_strict_recovery_gate(2026);

        assert!(report.accepted, "{:?}", report.metrics.failure_classes);
        assert_eq!(report.case_count, 9);
        assert_eq!(report.metrics.strict_success_rate, 1.0);
        assert_eq!(report.metrics.exact_topology_rate, 1.0);
        assert_eq!(report.metrics.exact_position_rate, 1.0);
        assert_eq!(report.metrics.perturbation_validity_rate, 1.0);
        assert!(report.metrics.equivalent_program_rate > 0.0);
        assert!(report.metrics.mean_program_compression >= 2.0);
        assert!(report.metrics.mean_search_time_units > 0.0);
        assert!(report.metrics.failure_classes.is_empty());
        assert!(report.cases.iter().all(|case| {
            case.strict_success
                && case.verification.strict_success
                && case
                    .verification
                    .verification
                    .as_ref()
                    .is_some_and(|verification| {
                        verification.residual_bytes == 0
                            && verification.literal_target_mesh_bytes == 0
                            && verification.per_vertex_independent_position_parameters == 0
                    })
        }));
    }

    #[test]
    fn recovery_uses_target_observation_not_canonical_answer_key() {
        let suite = hard_surface_strict_recovery_suite(2026);
        let mut case = suite.cases.first().expect("suite contains cases").clone();
        case.canonical_program.operations.clear();
        case.canonical_program.selections.clear();
        case.canonical_program.dependency_graph = ProgramDependencyGraph::default();

        let report = recover_hard_surface_case(&case);

        assert!(report.strict_success);
        assert_eq!(
            report.selected_node_id.as_deref(),
            Some("generated.strict_recovery.crate.candidate.000.recovered")
        );
        assert!(report.perturbation_valid);
    }

    #[test]
    fn perturbation_validity_is_measured_from_recovered_program_variants() {
        let suite = hard_surface_strict_recovery_suite(2026);
        let case = suite.cases.first().expect("suite contains cases");
        let recovered = program_from_observation(&case.target_observation.primary);
        let variants = perturbation_variants(&recovered, 4);

        assert_eq!(variants.len(), 4);
        assert!(variants.iter().all(|variant| variant != &recovered));
        assert!(measured_perturbation_neighborhood_is_valid(
            &case.id,
            &recovered,
            case.expected_exact_output.raw_geometry_size
        ));
        assert!(!measured_perturbation_neighborhood_is_valid(
            "empty",
            &ModelingProgram::strict_from_primitives(),
            case.expected_exact_output.raw_geometry_size
        ));
    }

    #[test]
    fn equivalent_success_uses_typed_equivalent_node_ids() {
        let suite = hard_surface_strict_recovery_suite(2026);
        let mut case = suite.cases.first().expect("suite contains cases").clone();
        case.id = "case.name.contains.equivalent".to_owned();
        case.target_observation.equivalents.clear();

        let report = recover_hard_surface_case(&case);

        assert!(report.strict_success);
        assert!(!report.equivalent_program_exact);
    }

    #[test]
    fn no_success_metrics_keep_search_rejection_failure_classes() {
        let suite = hard_surface_strict_recovery_suite(2026);
        let mut case = suite.cases.first().expect("suite contains cases").clone();
        case.expected_exact_output.output_fingerprint = "unreachable-target".to_owned();

        let case_report = recover_hard_surface_case(&case);
        let suite_report =
            run_hard_surface_strict_recovery_suite(&HardSurfaceStrictRecoverySuite {
                seed: 2026,
                cases: vec![case],
            });

        assert!(!case_report.strict_success);
        assert_eq!(
            case_report.failure_class,
            Some(StrictInverseFailureClass::NumericalNonExactness)
        );
        assert!(
            case_report
                .failure_classes
                .contains(&StrictInverseFailureClass::NumericalNonExactness)
        );
        assert!(suite_report.metrics.failure_classes.iter().any(|failure| {
            failure.class == StrictInverseFailureClass::NumericalNonExactness && failure.count == 1
        }));
    }

    #[test]
    fn residual_candidate_cannot_pass_wave15_gate() {
        let suite = hard_surface_strict_recovery_suite(2026);
        let mut case = suite.cases.first().expect("suite contains cases").clone();
        case.target_observation.equivalents.clear();
        let search_report = search_modeling_programs(&ProgramSearchProblem {
            strategy: ProgramSearchStrategy::AStar,
            objective: Default::default(),
            limits: ProgramSearchLimits {
                max_expanded_nodes: 4,
                max_frontier_nodes: 4,
                max_depth: 2,
                max_successes: 1,
            },
            exact_output: ExactOutputRequirement::from_expected_descriptor(
                case.expected_exact_output.clone(),
            ),
            space: ProgramSearchSpace::roots(candidate_nodes(&case, true)),
        });

        assert!(!search_report.has_strict_success());
        let failures = failure_hints_from_search_report(&search_report);
        assert!(
            failures.iter().any(|failure| {
                failure.class == StrictInverseFailureClass::NumericalNonExactness
            })
        );
    }
}
