//! Forward semantic runtime gate.
//!
//! This module composes the Wave 11-12 data contracts into one contract-level
//! forward runtime check. It still does not evaluate meshes; it proves that a
//! semantic program can be replayed deterministically, is admissible, has
//! complete stage provenance, and keeps Blender adapter caches out of the
//! canonical semantic result.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::corpus::{GeneratedModelingCorpus, GeneratedModelingCorpusCase};
use crate::deformation::{DeformationInferenceHint, deformation_operator_contract};
use crate::evaluator::{
    BlenderAdapterCacheEntry, BlenderAdapterCacheReport, EvaluationStageKind, EvaluatorConfig,
    EvaluatorOutputEnvelope, EvaluatorSemanticResult, ReplayTrace, build_replay_trace,
    semantic_output_fingerprint,
};
use crate::topology::{SelectionCount, SelectionSubject, topology_contract_for};
use crate::{
    ExplicitSelectionTarget, ModelingOperation, ModelingOperationKind, ModelingProgram,
    OperationPayloadKind, ProgramOperationId, SemanticAdmissibilityPolicy, SemanticSelection,
    SemanticSelectionId, SemanticSelectionPayload,
};

/// Runtime gate configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForwardRuntimeConfig {
    /// Canonical evaluator configuration used for replay.
    pub evaluator: EvaluatorConfig,
    /// Strict semantic admissibility policy.
    pub admissibility: SemanticAdmissibilityPolicy,
    /// Every operation must be present in a topology or deformation contract.
    pub require_supported_operation_contract: bool,
    /// Multi-operation programs must declare adjacent program-order operation edges.
    pub require_sequential_operation_dependency_edges: bool,
    /// Prove adapter-cache data cannot alter canonical semantic fingerprints.
    pub prove_adapter_cache_separation: bool,
}

impl ForwardRuntimeConfig {
    /// Canonical Shape Lab forward semantic runtime configuration.
    #[must_use]
    pub fn canonical() -> Self {
        Self {
            evaluator: EvaluatorConfig::canonical(),
            admissibility: SemanticAdmissibilityPolicy::strict(),
            require_supported_operation_contract: true,
            require_sequential_operation_dependency_edges: true,
            prove_adapter_cache_separation: true,
        }
    }
}

impl Default for ForwardRuntimeConfig {
    fn default() -> Self {
        Self::canonical()
    }
}

/// Runtime report for a corpus or program batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardRuntimeReport {
    /// True when all program reports are accepted and no batch issue exists.
    pub accepted: bool,
    /// Number of programs checked, including equivalent and ambiguous histories.
    pub program_count: usize,
    /// Number of operations checked.
    pub operation_count: usize,
    /// Per-program runtime reports.
    pub programs: Vec<ForwardProgramRuntimeReport>,
    /// Batch-level issues.
    pub issues: Vec<ForwardRuntimeIssue>,
}

impl ForwardRuntimeReport {
    fn from_programs(
        programs: Vec<ForwardProgramRuntimeReport>,
        issues: Vec<ForwardRuntimeIssue>,
    ) -> Self {
        let program_count = programs.len();
        let operation_count = programs.iter().map(|program| program.operation_count).sum();
        let accepted = issues.is_empty() && programs.iter().all(|program| program.accepted);

        Self {
            accepted,
            program_count,
            operation_count,
            programs,
            issues,
        }
    }
}

/// Runtime report for one modeling program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardProgramRuntimeReport {
    /// Stable program ID supplied by caller or corpus.
    pub program_id: String,
    /// True when every runtime gate passed.
    pub accepted: bool,
    /// Operation count.
    pub operation_count: usize,
    /// Trace fingerprint, when replay succeeded.
    pub trace_fingerprint: Option<String>,
    /// Canonical semantic result fingerprint, when replay succeeded.
    pub semantic_result_fingerprint: Option<String>,
    /// Replay produced complete input, per-operation, and final stage provenance.
    pub stage_provenance_complete: bool,
    /// Replaying the same program twice produced the same trace.
    pub deterministic_replay: bool,
    /// Adapter cache payloads are fingerprinted separately and cannot affect the semantic result.
    pub adapter_cache_separated: bool,
    /// Per-operation reports.
    pub operations: Vec<ForwardOperationRuntimeReport>,
    /// Program-level issues.
    pub issues: Vec<ForwardRuntimeIssue>,
}

impl ForwardProgramRuntimeReport {
    fn recompute_acceptance(&mut self) {
        self.accepted = self.issues.is_empty()
            && self
                .operations
                .iter()
                .all(ForwardOperationRuntimeReport::accepted)
            && self.stage_provenance_complete
            && self.deterministic_replay
            && self.adapter_cache_separated;
    }
}

/// Runtime report for one operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardOperationRuntimeReport {
    /// Operation ID.
    pub operation_id: ProgramOperationId,
    /// Operation kind.
    pub kind: ModelingOperationKind,
    /// The operation is covered by a forward topology or deformation contract.
    pub supported: bool,
    /// The operation passes strict semantic admissibility.
    pub semantically_admissible: bool,
    /// Topology selection arity and subjects match the operation contract.
    pub selection_contract_satisfied: bool,
}

impl ForwardOperationRuntimeReport {
    fn accepted(&self) -> bool {
        self.supported && self.semantically_admissible && self.selection_contract_satisfied
    }
}

/// Stable runtime issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardRuntimeIssue {
    /// Stable issue code.
    pub code: ForwardRuntimeIssueCode,
    /// Human-readable path.
    pub path: String,
    /// Human-readable message.
    pub message: String,
}

/// Stable issue codes for the forward semantic runtime gate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardRuntimeIssueCode {
    /// Canonical replay failed.
    ReplayFailed,
    /// Replaying the same semantic program twice changed the trace.
    ReplayNotDeterministic,
    /// Stage provenance was missing or inconsistent.
    StageProvenanceIncomplete,
    /// Operation has no forward semantic contract.
    UnsupportedOperation,
    /// Operation selection count does not match its contract.
    SelectionCountMismatch,
    /// Operation selection subject does not match its contract.
    SelectionSubjectMismatch,
    /// Operation references an undeclared selection.
    UnknownSelection,
    /// Explicit operation/selection dependency graph is incomplete or inconsistent.
    DependencyGraphIncomplete,
    /// Operation kind is forbidden by strict semantic policy.
    ForbiddenOperationKind,
    /// Payload kind is forbidden by strict semantic policy.
    ForbiddenPayloadKind,
    /// Parameter growth exceeds strict semantic policy.
    ParameterGrowthTooHigh,
    /// Perturbation validity is required but missing.
    MissingPerturbationValidity,
    /// Explicit selection payload is too large to be a semantic explanation.
    ExplicitSelectionPayloadTooLarge,
    /// Deformation parameters do not satisfy the compact deformation contract.
    DeformationParameterCountMismatch,
    /// Topology parameters do not satisfy the topology operator contract.
    TopologyParameterCountMismatch,
    /// Opaque correction or residual-like payload was attempted.
    OpaqueCorrectionUsed,
    /// A deformation contract is not admissible under the active policy.
    OperationContractInadmissible,
    /// Corpus exact-output descriptor does not satisfy strict exact replay.
    CorpusExpectedOutputNotExact,
    /// Corpus expected-output fingerprint does not match canonical replay.
    CorpusOutputFingerprintMismatch,
    /// Program is too large relative to the raw target descriptor.
    CompressionRatioTooLow,
    /// Adapter cache changed the canonical semantic result fingerprint.
    AdapterCacheAffectsSemanticOutput,
    /// Adapter cache fingerprints failed to remain independently distinguishable.
    AdapterCacheFingerprintNotSeparated,
}

/// Validate one modeling program against the complete forward runtime gate.
#[must_use]
pub fn validate_forward_program_runtime(
    program_id: impl Into<String>,
    program: &ModelingProgram,
    config: &ForwardRuntimeConfig,
) -> ForwardProgramRuntimeReport {
    let program_id = program_id.into();
    let selection_map = program
        .selections
        .iter()
        .map(|selection| (selection.id.clone(), selection))
        .collect::<BTreeMap<_, _>>();

    let mut issues = Vec::new();
    validate_operation_dependency_coverage(&program_id, program, config, &mut issues);
    validate_selection_dependency_coverage(&program_id, program, &mut issues);

    let operations = program
        .operations
        .iter()
        .map(|operation| {
            assess_operation_runtime(&program_id, operation, &selection_map, config, &mut issues)
        })
        .collect::<Vec<_>>();

    let mut trace_fingerprint = None;
    let mut semantic_result_fingerprint = None;
    let mut stage_provenance_complete = false;
    let mut deterministic_replay = false;
    let mut adapter_cache_separated = !config.prove_adapter_cache_separation;

    match build_replay_trace(program, &config.evaluator) {
        Ok(trace) => {
            trace_fingerprint = Some(trace.trace_fingerprint.clone());
            stage_provenance_complete =
                validate_stage_provenance(&program_id, program, &trace, &mut issues);
            deterministic_replay =
                validate_replay_determinism(&program_id, program, &trace, config, &mut issues);

            match semantic_output_fingerprint(program, &config.evaluator) {
                Ok(fingerprint) => {
                    semantic_result_fingerprint = Some(fingerprint);
                }
                Err(error) => push_issue(
                    &mut issues,
                    ForwardRuntimeIssueCode::ReplayFailed,
                    format!("{program_id}.semantic_result"),
                    format!("Failed to fingerprint semantic result: {error}."),
                ),
            }
            let semantic_result = EvaluatorSemanticResult::from_replay_trace(trace);
            if config.prove_adapter_cache_separation {
                adapter_cache_separated =
                    validate_adapter_cache_separation(&program_id, semantic_result, &mut issues);
            }
        }
        Err(error) => push_issue(
            &mut issues,
            ForwardRuntimeIssueCode::ReplayFailed,
            program_id.clone(),
            format!("Canonical replay failed: {error}."),
        ),
    }

    let mut report = ForwardProgramRuntimeReport {
        program_id,
        accepted: false,
        operation_count: program.operations.len(),
        trace_fingerprint,
        semantic_result_fingerprint,
        stage_provenance_complete,
        deterministic_replay,
        adapter_cache_separated,
        operations,
        issues,
    };
    report.recompute_acceptance();
    report
}

/// Validate a generated modeling corpus and all alternate histories.
#[must_use]
pub fn validate_generated_corpus_runtime(
    corpus: &GeneratedModelingCorpus,
    config: &ForwardRuntimeConfig,
) -> ForwardRuntimeReport {
    let mut programs = Vec::new();
    let issues = Vec::new();

    for case in &corpus.cases {
        programs.push(validate_corpus_program(
            case,
            format!("case.{}.canonical", case.id),
            &case.program,
            &case.expected_exact_output.output_fingerprint,
            CorpusFingerprintExpectation::MatchReplay,
            config,
        ));

        for history in &case.adversarial_equivalent_histories {
            programs.push(validate_corpus_program(
                case,
                format!("case.{}.history.{}", case.id, history.id),
                &history.program,
                &history.shared_output_fingerprint,
                CorpusFingerprintExpectation::MatchCanonicalCase,
                config,
            ));
        }

        for ambiguous in &case.ambiguous_programs {
            programs.push(validate_corpus_program(
                case,
                format!("case.{}.ambiguous.{}", case.id, ambiguous.id),
                &ambiguous.program,
                &ambiguous.expected_output_fingerprint,
                match ambiguous.expected_acceptance {
                    crate::corpus::AmbiguousProgramAcceptance::AcceptExactEquivalent => {
                        CorpusFingerprintExpectation::MatchCanonicalCase
                    }
                    crate::corpus::AmbiguousProgramAcceptance::RejectLowerSemanticSpecificity
                    | crate::corpus::AmbiguousProgramAcceptance::RankBelowCanonical => {
                        CorpusFingerprintExpectation::MatchReplay
                    }
                },
                config,
            ));
        }
    }

    ForwardRuntimeReport::from_programs(programs, issues)
}

fn validate_corpus_program(
    case: &GeneratedModelingCorpusCase,
    program_id: String,
    program: &ModelingProgram,
    expected_output_fingerprint: &str,
    fingerprint_expectation: CorpusFingerprintExpectation,
    config: &ForwardRuntimeConfig,
) -> ForwardProgramRuntimeReport {
    let mut report = validate_forward_program_runtime(program_id, program, config);
    if !case.expected_exact_output.is_strict_success_exact()
        || case.expected_exact_output.canonical_evaluator_version
            != config.evaluator.evaluator_version
    {
        push_issue(
            &mut report.issues,
            ForwardRuntimeIssueCode::CorpusExpectedOutputNotExact,
            format!("{}.expected_exact_output", report.program_id),
            "Corpus case does not declare exact zero-residual semantic output.",
        );
    }
    match fingerprint_expectation {
        CorpusFingerprintExpectation::MatchReplay => {
            if report.semantic_result_fingerprint.as_deref() != Some(expected_output_fingerprint) {
                push_issue(
                    &mut report.issues,
                    ForwardRuntimeIssueCode::CorpusOutputFingerprintMismatch,
                    format!(
                        "{}.expected_exact_output.output_fingerprint",
                        report.program_id
                    ),
                    "Corpus expected output fingerprint does not match canonical replay.",
                );
            }
        }
        CorpusFingerprintExpectation::MatchCanonicalCase => {
            if expected_output_fingerprint != case.expected_exact_output.output_fingerprint {
                push_issue(
                    &mut report.issues,
                    ForwardRuntimeIssueCode::CorpusOutputFingerprintMismatch,
                    format!(
                        "{}.expected_exact_output.output_fingerprint",
                        report.program_id
                    ),
                    "Corpus equivalent output fingerprint does not match the canonical case output.",
                );
            }
            if report.semantic_result_fingerprint.as_deref()
                != Some(&case.expected_exact_output.output_fingerprint)
            {
                push_issue(
                    &mut report.issues,
                    ForwardRuntimeIssueCode::CorpusOutputFingerprintMismatch,
                    format!(
                        "{}.expected_exact_output.output_fingerprint",
                        report.program_id
                    ),
                    "Corpus equivalent replay output does not match the canonical case output.",
                );
            }
        }
    }
    validate_corpus_compression_ratio(case, program, config, &mut report);
    report.recompute_acceptance();
    report
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CorpusFingerprintExpectation {
    MatchReplay,
    MatchCanonicalCase,
}

fn validate_corpus_compression_ratio(
    case: &GeneratedModelingCorpusCase,
    program: &ModelingProgram,
    config: &ForwardRuntimeConfig,
    report: &mut ForwardProgramRuntimeReport,
) {
    match program.description_size_bytes() {
        Ok(program_description_size) => {
            let compression_ratio = if program_description_size == 0 {
                0.0
            } else {
                case.target_mesh.raw_geometry_size.total_bytes() as f64
                    / program_description_size as f64
            };
            if compression_ratio < config.admissibility.minimum_compression_ratio {
                push_issue(
                    &mut report.issues,
                    ForwardRuntimeIssueCode::CompressionRatioTooLow,
                    format!("{}.compression_ratio", report.program_id),
                    format!(
                        "Program compression ratio {compression_ratio:.3} is below policy minimum {:.3}.",
                        config.admissibility.minimum_compression_ratio
                    ),
                );
            }
        }
        Err(error) => push_issue(
            &mut report.issues,
            ForwardRuntimeIssueCode::ReplayFailed,
            format!("{}.program_description_size", report.program_id),
            format!("Failed to measure program description size: {error}."),
        ),
    }
}

fn assess_operation_runtime(
    program_id: &str,
    operation: &ModelingOperation,
    selections: &BTreeMap<SemanticSelectionId, &SemanticSelection>,
    config: &ForwardRuntimeConfig,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> ForwardOperationRuntimeReport {
    let operation_path = format!("{program_id}.operations.{}", operation.id.0);
    let supported = validate_operation_support(
        &operation_path,
        operation,
        config.require_supported_operation_contract,
        issues,
    );
    let selection_contract_satisfied =
        validate_selection_contract(&operation_path, operation, selections, issues);
    let semantically_admissible =
        validate_operation_admissibility(&operation_path, operation, selections, config, issues);

    ForwardOperationRuntimeReport {
        operation_id: operation.id.clone(),
        kind: operation.kind,
        supported,
        semantically_admissible,
        selection_contract_satisfied,
    }
}

fn validate_operation_support(
    operation_path: &str,
    operation: &ModelingOperation,
    require_supported_operation_contract: bool,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    let topology_supported = topology_contract_for(operation.kind).is_some();
    let deformation_supported = deformation_operator_contract(operation.kind).is_some();
    let supported = topology_supported || deformation_supported;
    if require_supported_operation_contract && !supported {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::UnsupportedOperation,
            operation_path,
            format!(
                "Operation kind {:?} has no forward semantic runtime contract.",
                operation.kind
            ),
        );
    }
    supported || !require_supported_operation_contract
}

fn validate_selection_contract(
    operation_path: &str,
    operation: &ModelingOperation,
    selections: &BTreeMap<SemanticSelectionId, &SemanticSelection>,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    if let Some(contract) = topology_contract_for(operation.kind) {
        let mut valid = validate_selection_count(
            contract.selection_requirements.count,
            operation.selections.len(),
        );
        if !valid {
            push_issue(
                issues,
                ForwardRuntimeIssueCode::SelectionCountMismatch,
                format!("{operation_path}.selections"),
                format!(
                    "Operation has {} selections, expected {:?}.",
                    operation.selections.len(),
                    contract.selection_requirements.count
                ),
            );
        }

        let enforce_positional_subjects = operation.kind
            == ModelingOperationKind::ConstrainedBoolean
            && contract.selection_requirements.ordered
            && contract.selection_requirements.accepted_subjects.len()
                == operation.selections.len()
            && contract.selection_requirements.accepted_subjects.len() > 1;
        for (index, selection_id) in operation.selections.iter().enumerate() {
            let Some(selection) = selections.get(selection_id) else {
                valid = false;
                push_issue(
                    issues,
                    ForwardRuntimeIssueCode::UnknownSelection,
                    format!("{operation_path}.selections"),
                    format!("Selection {} is not declared.", selection_id.0),
                );
                continue;
            };
            let accepted_subjects = if enforce_positional_subjects {
                &contract.selection_requirements.accepted_subjects[index..=index]
            } else {
                &contract.selection_requirements.accepted_subjects
            };
            if !selection_satisfies_any_subject(selection, accepted_subjects) {
                valid = false;
                push_issue(
                    issues,
                    ForwardRuntimeIssueCode::SelectionSubjectMismatch,
                    format!("{operation_path}.selections.{}", selection_id.0),
                    format!(
                        "Selection subjects {:?} do not satisfy {:?}.",
                        selection_subjects(selection),
                        accepted_subjects
                    ),
                );
            }
        }
        return valid;
    }

    if let Some(contract) = deformation_operator_contract(operation.kind) {
        let accepted_subjects = deformation_selection_subjects(&contract.inference_hints);
        if accepted_subjects.is_empty() {
            return true;
        }
        let mut valid = operation.selections.len() == 1;
        if !valid {
            push_issue(
                issues,
                ForwardRuntimeIssueCode::SelectionCountMismatch,
                format!("{operation_path}.selections"),
                format!(
                    "Deformation operation has {} selections, expected exactly one semantic target.",
                    operation.selections.len()
                ),
            );
        }
        for selection_id in &operation.selections {
            let Some(selection) = selections.get(selection_id) else {
                valid = false;
                push_issue(
                    issues,
                    ForwardRuntimeIssueCode::UnknownSelection,
                    format!("{operation_path}.selections"),
                    format!("Selection {} is not declared.", selection_id.0),
                );
                continue;
            };
            if !selection_satisfies_any_subject(selection, &accepted_subjects) {
                valid = false;
                push_issue(
                    issues,
                    ForwardRuntimeIssueCode::SelectionSubjectMismatch,
                    format!("{operation_path}.selections.{}", selection_id.0),
                    format!(
                        "Deformation target subjects {:?} do not satisfy {:?}.",
                        selection_subjects(selection),
                        accepted_subjects
                    ),
                );
            }
        }
        return valid;
    }

    true
}

fn deformation_selection_subjects(hints: &[DeformationInferenceHint]) -> Vec<SelectionSubject> {
    let mut subjects = Vec::new();
    if hints.contains(&DeformationInferenceHint::SemanticPartSelection) {
        subjects.push(SelectionSubject::Part);
    }
    if hints.contains(&DeformationInferenceHint::SemanticRegionSelection) {
        subjects.push(SelectionSubject::Region);
    }
    subjects
}

fn selection_satisfies_any_subject(
    selection: &SemanticSelection,
    accepted_subjects: &[SelectionSubject],
) -> bool {
    let subjects = selection_subjects(selection);
    subjects
        .iter()
        .any(|subject| accepted_subjects.contains(subject))
}

fn operation_semantic_parameter_count(parameters: &[crate::SemanticParameter]) -> usize {
    parameters.iter().map(semantic_parameter_width).sum()
}

fn semantic_parameter_width(parameter: &crate::SemanticParameter) -> usize {
    match parameter {
        crate::SemanticParameter::Scalar { .. }
        | crate::SemanticParameter::Integer { .. }
        | crate::SemanticParameter::Boolean { .. }
        | crate::SemanticParameter::Choice { .. } => 1,
        crate::SemanticParameter::Vector3 { .. } => 3,
        crate::SemanticParameter::Quaternion { .. } => 4,
    }
}

fn validate_deformation_parameter_count(
    operation_path: &str,
    operation: &ModelingOperation,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    let Some(contract) = deformation_operator_contract(operation.kind) else {
        return true;
    };
    let count = operation_semantic_parameter_count(&operation.parameters);
    let minimum = usize::from(contract.semantic_parameter_count.minimum);
    let maximum = contract
        .semantic_parameter_count
        .maximum
        .map(usize::from)
        .unwrap_or(usize::MAX);
    let valid = count >= minimum && count <= maximum;
    if !valid {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::DeformationParameterCountMismatch,
            format!("{operation_path}.parameters"),
            format!(
                "Deformation operation has {count} scalar-equivalent parameters, expected {minimum}..={maximum}."
            ),
        );
    }
    valid
}

fn validate_declared_selections_exist(
    operation_path: &str,
    operation: &ModelingOperation,
    selections: &BTreeMap<SemanticSelectionId, &SemanticSelection>,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    let mut valid = true;
    for selection_id in &operation.selections {
        let Some(selection) = selections.get(selection_id) else {
            valid = false;
            push_issue(
                issues,
                ForwardRuntimeIssueCode::UnknownSelection,
                format!("{operation_path}.selections"),
                format!("Selection {} is not declared.", selection_id.0),
            );
            continue;
        };
        let _ = selection;
    }
    valid
}

fn validate_operation_admissibility(
    operation_path: &str,
    operation: &ModelingOperation,
    selections: &BTreeMap<SemanticSelectionId, &SemanticSelection>,
    config: &ForwardRuntimeConfig,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    let mut admissible = true;
    let policy = &config.admissibility;

    if policy.forbidden_operation_kinds.contains(&operation.kind) {
        admissible = false;
        push_issue(
            issues,
            ForwardRuntimeIssueCode::ForbiddenOperationKind,
            operation_path,
            format!("Operation kind {:?} is forbidden.", operation.kind),
        );
        push_issue(
            issues,
            ForwardRuntimeIssueCode::OpaqueCorrectionUsed,
            operation_path,
            "Forbidden operation would act as an opaque correction.",
        );
    }

    if let Some(contract) = deformation_operator_contract(operation.kind) {
        if !contract.is_admissible_under(policy) {
            admissible = false;
            push_issue(
                issues,
                ForwardRuntimeIssueCode::OperationContractInadmissible,
                operation_path,
                format!(
                    "Deformation contract for {:?} is not admissible under policy.",
                    operation.kind
                ),
            );
        }
        for payload in &operation.payloads {
            if !contract.allowed_payload_kinds.contains(&payload.kind) {
                admissible = false;
                push_issue(
                    issues,
                    ForwardRuntimeIssueCode::ForbiddenPayloadKind,
                    operation_path,
                    format!(
                        "Payload kind {:?} is not allowed for {:?}.",
                        payload.kind, operation.kind
                    ),
                );
            }
        }
    }
    admissible &= validate_topology_parameter_count(operation_path, operation, issues);
    admissible &= validate_deformation_parameter_count(operation_path, operation, issues);

    if !check_parameter_growth(
        operation_semantic_parameter_count(&operation.parameters),
        operation.affected_element_count,
        policy.maximum_parameter_growth_relative_to_affected_elements,
    ) {
        admissible = false;
        push_issue(
            issues,
            ForwardRuntimeIssueCode::ParameterGrowthTooHigh,
            format!("{operation_path}.parameters"),
            "Direct operation parameter growth exceeds semantic policy.",
        );
    }

    for payload in &operation.payloads {
        if policy
            .forbidden_opaque_payload_kinds
            .contains(&payload.kind)
        {
            admissible = false;
            push_issue(
                issues,
                ForwardRuntimeIssueCode::ForbiddenPayloadKind,
                format!("{operation_path}.payloads"),
                format!("Payload kind {:?} is forbidden.", payload.kind),
            );
            push_issue(
                issues,
                ForwardRuntimeIssueCode::OpaqueCorrectionUsed,
                format!("{operation_path}.payloads"),
                "Forbidden payload would act as an opaque correction.",
            );
        }
        if payload.kind == OperationPayloadKind::ExplicitSelectionIndices
            && payload.affected_element_count > policy.maximum_explicit_selection_payload
        {
            admissible = false;
            push_issue(
                issues,
                ForwardRuntimeIssueCode::ExplicitSelectionPayloadTooLarge,
                format!("{operation_path}.payloads"),
                format!(
                    "Explicit selection payload declares {} element IDs.",
                    payload.affected_element_count
                ),
            );
        }
        if !check_parameter_growth(
            payload.semantic_parameter_count,
            payload.affected_element_count,
            policy.maximum_parameter_growth_relative_to_affected_elements,
        ) {
            admissible = false;
            push_issue(
                issues,
                ForwardRuntimeIssueCode::ParameterGrowthTooHigh,
                format!("{operation_path}.payloads"),
                "Payload parameter growth exceeds semantic policy.",
            );
        }
        if policy.perturbation_validity_required && !payload.perturbation_valid {
            admissible = false;
            push_issue(
                issues,
                ForwardRuntimeIssueCode::MissingPerturbationValidity,
                format!("{operation_path}.payloads"),
                "Payload does not declare perturbation validity.",
            );
        }
    }

    if !validate_declared_selections_exist(operation_path, operation, selections, issues) {
        admissible = false;
    }
    for selection_id in &operation.selections {
        let Some(selection) = selections.get(selection_id) else {
            continue;
        };
        let explicit_len = selection.explicit_payload_len();
        if explicit_len > policy.maximum_explicit_selection_payload {
            admissible = false;
            push_issue(
                issues,
                ForwardRuntimeIssueCode::ExplicitSelectionPayloadTooLarge,
                format!("{operation_path}.selections.{}", selection_id.0),
                format!("Explicit selection contains {explicit_len} element IDs."),
            );
        }
    }

    admissible
}

fn validate_topology_parameter_count(
    operation_path: &str,
    operation: &ModelingOperation,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    let Some(contract) = topology_contract_for(operation.kind) else {
        return true;
    };
    let count = operation_semantic_parameter_count(&operation.parameters);
    let valid = count == contract.semantic_parameter_count;
    if !valid {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::TopologyParameterCountMismatch,
            format!("{operation_path}.parameters"),
            format!(
                "Topology operation has {count} scalar-equivalent parameters, expected {}.",
                contract.semantic_parameter_count
            ),
        );
    }
    valid
}

fn validate_operation_dependency_coverage(
    program_id: &str,
    program: &ModelingProgram,
    config: &ForwardRuntimeConfig,
    issues: &mut Vec<ForwardRuntimeIssue>,
) {
    if !config.require_sequential_operation_dependency_edges {
        return;
    }

    let operation_edges = program
        .dependency_graph
        .operation_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    for pair in program.operations.windows(2) {
        let required_edge = (pair[0].id.clone(), pair[1].id.clone());
        if !operation_edges.contains(&required_edge) {
            push_issue(
                issues,
                ForwardRuntimeIssueCode::DependencyGraphIncomplete,
                format!("{program_id}.dependency_graph.operation_edges"),
                format!(
                    "Missing adjacent operation dependency {} -> {}.",
                    required_edge.0.0, required_edge.1.0
                ),
            );
        }
    }
}

fn validate_selection_dependency_coverage(
    program_id: &str,
    program: &ModelingProgram,
    issues: &mut Vec<ForwardRuntimeIssue>,
) {
    let selection_edges = program
        .dependency_graph
        .selection_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let operation_selections = program
        .operations
        .iter()
        .flat_map(|operation| {
            operation
                .selections
                .iter()
                .cloned()
                .map(|selection| (selection, operation.id.clone()))
        })
        .collect::<BTreeSet<_>>();

    for edge in operation_selections.difference(&selection_edges) {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::DependencyGraphIncomplete,
            format!("{program_id}.dependency_graph.selection_edges"),
            format!(
                "Missing explicit selection dependency {} -> {}.",
                edge.0.0, edge.1.0
            ),
        );
    }
    for edge in selection_edges.difference(&operation_selections) {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::DependencyGraphIncomplete,
            format!("{program_id}.dependency_graph.selection_edges"),
            format!(
                "Selection dependency {} -> {} is not used by the operation.",
                edge.0.0, edge.1.0
            ),
        );
    }
}

fn validate_stage_provenance(
    program_id: &str,
    program: &ModelingProgram,
    trace: &ReplayTrace,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    let expected_stage_count = program.operations.len() + 2;
    let mut complete = trace.stage_fingerprints.len() == expected_stage_count
        && trace
            .stage_fingerprints
            .first()
            .is_some_and(|stage| stage.stage == EvaluationStageKind::InputProgram)
        && trace
            .stage_fingerprints
            .last()
            .is_some_and(|stage| stage.stage == EvaluationStageKind::SemanticResult)
        && trace.operations.len() == program.operations.len();

    for (index, operation) in program.operations.iter().enumerate() {
        let stage = trace.stage_fingerprints.get(index + 1);
        let trace_operation = trace.operations.get(index);
        let operation_complete = stage.is_some_and(|stage| {
            stage.stage == EvaluationStageKind::Operation
                && stage.operation_index == Some(index)
                && stage.operation_id.as_ref() == Some(&operation.id)
                && !stage.fingerprint.is_empty()
        }) && trace_operation.is_some_and(|trace_operation| {
            trace_operation.operation_index == index
                && trace_operation.operation_id == operation.id
                && trace_operation.operation_kind == operation.kind
                && !trace_operation.input_stage_fingerprint.is_empty()
                && !trace_operation.output_stage_fingerprint.is_empty()
        });
        complete &= operation_complete;
    }

    if !complete {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::StageProvenanceIncomplete,
            program_id,
            "Replay trace is missing complete input, operation, or final stage provenance.",
        );
    }

    complete
}

fn validate_replay_determinism(
    program_id: &str,
    program: &ModelingProgram,
    first_trace: &ReplayTrace,
    config: &ForwardRuntimeConfig,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    match build_replay_trace(program, &config.evaluator) {
        Ok(second_trace) if &second_trace == first_trace => true,
        Ok(_) => {
            push_issue(
                issues,
                ForwardRuntimeIssueCode::ReplayNotDeterministic,
                program_id,
                "Replaying the same program produced a different trace.",
            );
            false
        }
        Err(error) => {
            push_issue(
                issues,
                ForwardRuntimeIssueCode::ReplayFailed,
                program_id,
                format!("Second canonical replay failed: {error}."),
            );
            false
        }
    }
}

fn validate_adapter_cache_separation(
    program_id: &str,
    semantic_result: EvaluatorSemanticResult,
    issues: &mut Vec<ForwardRuntimeIssue>,
) -> bool {
    let first = EvaluatorOutputEnvelope {
        semantic_result: semantic_result.clone(),
        blender_adapter_cache: Some(cache_report("adapter-cache-a", "aaaaaaaa")),
    };
    let second = EvaluatorOutputEnvelope {
        semantic_result,
        blender_adapter_cache: Some(cache_report("adapter-cache-b", "bbbbbbbb")),
    };

    let first_semantic = first.semantic_result_fingerprint();
    let second_semantic = second.semantic_result_fingerprint();
    let first_cache = first.adapter_cache_fingerprint();
    let second_cache = second.adapter_cache_fingerprint();
    let semantic_equal = first_semantic.ok() == second_semantic.ok();
    let cache_distinct = first_cache.ok().flatten() != second_cache.ok().flatten();

    if !semantic_equal {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::AdapterCacheAffectsSemanticOutput,
            program_id,
            "Changing Blender adapter cache data changed the semantic result fingerprint.",
        );
    }
    if !cache_distinct {
        push_issue(
            issues,
            ForwardRuntimeIssueCode::AdapterCacheFingerprintNotSeparated,
            program_id,
            "Adapter cache fingerprints were not independently distinguishable.",
        );
    }

    semantic_equal && cache_distinct
}

fn cache_report(key: &str, cache_fingerprint: &str) -> BlenderAdapterCacheReport {
    BlenderAdapterCacheReport {
        adapter_version: "shape-program-runtime-cache-probe".to_owned(),
        entries: vec![BlenderAdapterCacheEntry {
            key: key.to_owned(),
            source_stage_fingerprint: "semantic-stage".to_owned(),
            cache_fingerprint: cache_fingerprint.to_owned(),
            byte_len: 64,
        }],
    }
}

fn validate_selection_count(count: SelectionCount, actual: usize) -> bool {
    match count {
        SelectionCount::ExactlyZero => actual == 0,
        SelectionCount::ExactlyOne => actual == 1,
        SelectionCount::ExactlyTwo => actual == 2,
        SelectionCount::OneOrMore => actual >= 1,
        SelectionCount::TwoOrMore => actual >= 2,
    }
}

fn check_parameter_growth(
    semantic_parameter_count: usize,
    affected_element_count: usize,
    maximum_ratio: f64,
) -> bool {
    if affected_element_count == 0 {
        semantic_parameter_count == 0
    } else {
        (semantic_parameter_count as f64 / affected_element_count as f64) <= maximum_ratio
    }
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
        SemanticSelectionPayload::BooleanOperand { .. } => vec![SelectionSubject::BooleanOperand],
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

fn push_issue(
    issues: &mut Vec<ForwardRuntimeIssue>,
    code: ForwardRuntimeIssueCode,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(ForwardRuntimeIssue {
        code,
        path: path.into(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use crate::corpus::generated_modeling_corpus;
    use crate::deformation::all_deformation_operator_contracts;
    use crate::topology::{all_topology_contracts, topology_contract_for};
    use crate::{
        ExplicitSelectionTarget, ModelingOperation, OperationPayloadDescriptor,
        ProgramDependencyGraph, RawGeometrySize, SemanticBoundaryLoopId, SemanticParameter,
        SemanticPartId, SemanticRegionId, SemanticSelectionPayload,
    };

    use super::*;

    #[test]
    fn expanded_generated_corpus_passes_forward_runtime_gate() {
        let corpus = generated_modeling_corpus(2026);
        let report = validate_generated_corpus_runtime(&corpus, &ForwardRuntimeConfig::canonical());

        assert!(report.accepted, "{:#?}", report);
        assert!(report.program_count >= corpus.cases.len());
        assert!(report.operation_count >= 4);
    }

    #[test]
    fn generated_corpus_covers_required_forward_families() {
        let corpus = generated_modeling_corpus(2026);
        let families = corpus
            .cases
            .iter()
            .map(|case| case.target_mesh.family.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(families, BTreeSet::from(["box_primitive"]));
    }

    #[test]
    fn every_supported_operator_has_a_replayable_runtime_fixture() {
        let mut supported_kinds = all_topology_contracts()
            .into_iter()
            .map(|contract| contract.kind)
            .collect::<BTreeSet<_>>();
        supported_kinds.extend(
            all_deformation_operator_contracts()
                .into_iter()
                .map(|contract| contract.kind),
        );

        for kind in supported_kinds {
            let program = fixture_program_for(kind);
            let report = validate_forward_program_runtime(
                format!("fixture.{kind:?}"),
                &program,
                &ForwardRuntimeConfig::canonical(),
            );
            assert!(report.accepted, "{kind:?}: {:#?}", report);
        }
    }

    #[test]
    fn opaque_correction_is_rejected_by_runtime_gate() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(modeling_operation(
            "op.cheat",
            ModelingOperationKind::OpaqueResidual,
            Vec::new(),
            Vec::new(),
            1,
        ));

        let report = validate_forward_program_runtime(
            "opaque.cheat",
            &program,
            &ForwardRuntimeConfig::canonical(),
        );

        assert!(!report.accepted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == ForwardRuntimeIssueCode::OpaqueCorrectionUsed
                || issue.code == ForwardRuntimeIssueCode::UnsupportedOperation
        }));
    }

    #[test]
    fn runtime_rejects_missing_operation_dependency_edge() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(modeling_operation(
            "op.first",
            ModelingOperationKind::PrimitiveCreate,
            Vec::new(),
            Vec::new(),
            1,
        ));
        program.operations.push(modeling_operation(
            "op.second",
            ModelingOperationKind::RegionInset,
            Vec::new(),
            Vec::new(),
            1,
        ));

        let report = validate_forward_program_runtime(
            "missing.operation.edge",
            &program,
            &ForwardRuntimeConfig::canonical(),
        );

        assert!(!report.accepted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == ForwardRuntimeIssueCode::DependencyGraphIncomplete
                && issue.path.ends_with("dependency_graph.operation_edges")
        }));
    }

    #[test]
    fn runtime_rejects_reversed_ordered_boolean_operands() {
        let mut program = ModelingProgram::strict_from_primitives();
        let host = selection_for_subject("sel.host", SelectionSubject::Part);
        let operand = selection_for_subject("sel.operand", SelectionSubject::BooleanOperand);
        program.selections = vec![host.clone(), operand.clone()];
        program.operations.push(modeling_operation(
            "op.boolean",
            ModelingOperationKind::ConstrainedBoolean,
            vec![operand.id, host.id],
            vec![SemanticParameter::Choice {
                name: "operation".to_owned(),
                value: "subtract".to_owned(),
            }],
            24,
        ));
        program.dependency_graph.selection_edges = program
            .operations
            .iter()
            .flat_map(|operation| {
                operation
                    .selections
                    .iter()
                    .cloned()
                    .map(|selection| (selection, operation.id.clone()))
            })
            .collect();

        let report = validate_forward_program_runtime(
            "reversed.boolean.operands",
            &program,
            &ForwardRuntimeConfig::canonical(),
        );

        assert!(!report.accepted);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == ForwardRuntimeIssueCode::SelectionSubjectMismatch })
        );
    }

    #[test]
    fn runtime_rejects_deformation_without_target_or_compact_controls() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(modeling_operation(
            "op.bad_bend",
            ModelingOperationKind::Bend,
            Vec::new(),
            vec![SemanticParameter::Scalar {
                name: "amount".to_owned(),
                value: 0.1,
            }],
            32,
        ));

        let report = validate_forward_program_runtime(
            "bad.deformation",
            &program,
            &ForwardRuntimeConfig::canonical(),
        );

        assert!(!report.accepted);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == ForwardRuntimeIssueCode::SelectionCountMismatch })
        );
        assert!(report.issues.iter().any(|issue| {
            issue.code == ForwardRuntimeIssueCode::DeformationParameterCountMismatch
        }));
    }

    #[test]
    fn runtime_rejects_topology_parameter_count_mismatch() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(modeling_operation(
            "op.underdescribed_create",
            ModelingOperationKind::PrimitiveCreate,
            Vec::new(),
            vec![SemanticParameter::Scalar {
                name: "width".to_owned(),
                value: 1.0,
            }],
            32,
        ));

        let report = validate_forward_program_runtime(
            "bad.topology.parameters",
            &program,
            &ForwardRuntimeConfig::canonical(),
        );

        assert!(!report.accepted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == ForwardRuntimeIssueCode::TopologyParameterCountMismatch
        }));
    }

    fn fixture_program_for(kind: ModelingOperationKind) -> ModelingProgram {
        let mut program = ModelingProgram::strict_from_primitives();
        let (selections, selection_ids) = fixture_selections_for(kind);
        program.selections = selections;
        program.operations.push(modeling_operation(
            "op.fixture",
            kind,
            selection_ids,
            fixture_parameters_for(kind),
            64,
        ));
        program.dependency_graph = ProgramDependencyGraph {
            operation_edges: Vec::new(),
            selection_edges: program
                .operations
                .iter()
                .flat_map(|operation| {
                    operation
                        .selections
                        .iter()
                        .cloned()
                        .map(|selection| (selection, operation.id.clone()))
                })
                .collect(),
        };
        program.dependency_graph.selection_edges.sort();
        program.dependency_graph.selection_edges.dedup();
        program
    }

    fn fixture_selections_for(
        kind: ModelingOperationKind,
    ) -> (Vec<SemanticSelection>, Vec<SemanticSelectionId>) {
        if let Some(contract) = topology_contract_for(kind) {
            return match contract.selection_requirements.count {
                SelectionCount::ExactlyZero => (Vec::new(), Vec::new()),
                SelectionCount::ExactlyOne | SelectionCount::OneOrMore => {
                    let selection = selection_for_subject(
                        "sel.fixture.0",
                        contract.selection_requirements.accepted_subjects[0],
                    );
                    (vec![selection.clone()], vec![selection.id])
                }
                SelectionCount::ExactlyTwo | SelectionCount::TwoOrMore => {
                    let first = selection_for_subject(
                        "sel.fixture.0",
                        contract.selection_requirements.accepted_subjects[0],
                    );
                    let second_subject = *contract
                        .selection_requirements
                        .accepted_subjects
                        .get(1)
                        .unwrap_or(&contract.selection_requirements.accepted_subjects[0]);
                    let second = selection_for_subject("sel.fixture.1", second_subject);
                    (
                        vec![first.clone(), second.clone()],
                        vec![first.id, second.id],
                    )
                }
            };
        }

        let Some(contract) = deformation_operator_contract(kind) else {
            return (Vec::new(), Vec::new());
        };
        let subjects = deformation_selection_subjects(&contract.inference_hints);
        let Some(subject) = subjects.first().copied() else {
            return (Vec::new(), Vec::new());
        };
        let selection = selection_for_subject("sel.fixture.0", subject);
        (vec![selection.clone()], vec![selection.id])
    }

    fn selection_for_subject(id: &str, subject: SelectionSubject) -> SemanticSelection {
        SemanticSelection {
            id: SemanticSelectionId(id.to_owned()),
            payload: match subject {
                SelectionSubject::Part | SelectionSubject::Object => {
                    SemanticSelectionPayload::Part {
                        part: SemanticPartId(format!("{id}.part")),
                    }
                }
                SelectionSubject::Region => SemanticSelectionPayload::Region {
                    region: SemanticRegionId(format!("{id}.region")),
                },
                SelectionSubject::BoundaryLoop => SemanticSelectionPayload::BoundaryLoop {
                    boundary_loop: SemanticBoundaryLoopId(format!("{id}.loop")),
                },
                SelectionSubject::EdgeLoop | SelectionSubject::EdgeSet => {
                    SemanticSelectionPayload::EdgeClass {
                        class: format!("{id}.edge_class"),
                    }
                }
                SelectionSubject::VertexSet => SemanticSelectionPayload::ExplicitIndices {
                    target: ExplicitSelectionTarget::Vertex,
                    indices: vec![0, 1],
                },
                SelectionSubject::BooleanOperand => SemanticSelectionPayload::BooleanOperand {
                    operand_id: format!("{id}.operand"),
                },
            },
        }
    }

    fn fixture_parameters_for(kind: ModelingOperationKind) -> Vec<SemanticParameter> {
        if let Some(contract) = topology_contract_for(kind) {
            return parameters_for_scalar_equivalent_count(contract.semantic_parameter_count);
        }

        if let Some(contract) = deformation_operator_contract(kind) {
            return parameters_for_scalar_equivalent_count(usize::from(
                contract.semantic_parameter_count.minimum,
            ));
        }

        match kind {
            ModelingOperationKind::PartTransform | ModelingOperationKind::RegionTransform => {
                vec![
                    SemanticParameter::Vector3 {
                        name: "translation".to_owned(),
                        value: [0.0, 0.0, 0.0],
                    },
                    SemanticParameter::Quaternion {
                        name: "rotation".to_owned(),
                        value: [0.0, 0.0, 0.0, 1.0],
                    },
                    SemanticParameter::Vector3 {
                        name: "scale".to_owned(),
                        value: [1.0, 1.0, 1.0],
                    },
                ]
            }
            _ => vec![SemanticParameter::Scalar {
                name: "amount".to_owned(),
                value: 0.125,
            }],
        }
    }

    fn parameters_for_scalar_equivalent_count(count: usize) -> Vec<SemanticParameter> {
        let mut parameters = Vec::new();
        let mut remaining = count;
        while remaining >= 3 {
            parameters.push(SemanticParameter::Vector3 {
                name: format!("vector_control_{}", parameters.len()),
                value: [0.0, 0.0, 0.0],
            });
            remaining -= 3;
        }
        for index in 0..remaining {
            parameters.push(SemanticParameter::Scalar {
                name: format!("scalar_control_{index}"),
                value: 0.0,
            });
        }
        parameters
    }

    fn modeling_operation(
        id: &str,
        kind: ModelingOperationKind,
        selections: Vec<SemanticSelectionId>,
        parameters: Vec<SemanticParameter>,
        affected_element_count: usize,
    ) -> ModelingOperation {
        let parameter_count = operation_semantic_parameter_count(&parameters);
        ModelingOperation {
            id: ProgramOperationId(id.to_owned()),
            kind,
            selections,
            parameters,
            affected_element_count,
            payloads: vec![OperationPayloadDescriptor {
                kind: OperationPayloadKind::SemanticParameters,
                encoded_bytes: parameter_count * 16,
                semantic_parameter_count: parameter_count,
                affected_element_count,
                perturbation_valid: true,
            }],
        }
    }

    #[allow(dead_code)]
    fn raw_size() -> RawGeometrySize {
        RawGeometrySize {
            vertex_count: 256,
            face_count: 192,
            position_bytes: 256 * 3 * 8,
            topology_bytes: 192 * 4 * 4,
        }
    }
}
