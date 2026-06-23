//! Canonical deterministic evaluator contract.
//!
//! This module defines replay, fingerprinting, and adapter-cache boundaries for
//! forward evaluators. It intentionally does not evaluate meshes yet.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    BaseTopologyReference, CANONICAL_EVALUATOR_VERSION, ExplicitSelectionTarget, GrammarProfile,
    MODELING_PROGRAM_SCHEMA_VERSION, ModelingOperation, ModelingOperationKind, ModelingProgram,
    OperationPayloadDescriptor, ProgramDependencyGraph, ProgramOperationId, SemanticBoundaryLoopId,
    SemanticParameter, SemanticPartId, SemanticRegionId, SemanticSelection, SemanticSelectionId,
    SemanticSelectionPayload, SpatialPrimitiveSelection,
};

/// Current schema version for evaluator contracts.
pub const EVALUATOR_CONTRACT_SCHEMA_VERSION: u32 = 1;

const SEMANTIC_RESULT_FINGERPRINT_DOMAIN: &str = "shape-program.evaluator.semantic-result.v1";
const SEMANTIC_OUTPUT_FINGERPRINT_DOMAIN: &str = "shape-program.evaluator.semantic-output.v1";
const STAGE_FINGERPRINT_DOMAIN: &str = "shape-program.evaluator.stage.v1";
const TRACE_FINGERPRINT_DOMAIN: &str = "shape-program.evaluator.trace.v1";
const ADAPTER_CACHE_FINGERPRINT_DOMAIN: &str = "shape-program.evaluator.adapter-cache.v1";

/// Serializable deterministic evaluator configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorConfig {
    /// Evaluator contract schema version.
    pub schema_version: u32,
    /// Canonical evaluator implementation contract required for exact replay.
    pub evaluator_version: String,
    /// Canonical float normalization and serialization rules.
    pub float_rules: CanonicalFloatRules,
    /// Operation ordering rule used by replay traces.
    pub operation_ordering: OperationOrderingRule,
    /// Stage fingerprint policy.
    pub stage_fingerprints: StageFingerprintPolicy,
    /// Replay trace policy.
    pub replay_trace: ReplayTracePolicy,
    /// Adapter-cache separation policy.
    pub adapter_cache: AdapterCachePolicy,
}

impl EvaluatorConfig {
    /// Canonical deterministic evaluator configuration for this contract crate.
    #[must_use]
    pub fn canonical() -> Self {
        Self {
            schema_version: EVALUATOR_CONTRACT_SCHEMA_VERSION,
            evaluator_version: CANONICAL_EVALUATOR_VERSION.to_owned(),
            float_rules: CanonicalFloatRules::default(),
            operation_ordering: OperationOrderingRule::ProgramOrderStableIdsProducerBeforeConsumer,
            stage_fingerprints: StageFingerprintPolicy::default(),
            replay_trace: ReplayTracePolicy::default(),
            adapter_cache: AdapterCachePolicy::default(),
        }
    }
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self::canonical()
    }
}

/// Canonical float normalization policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalFloatRules {
    /// Reject NaN and infinite values.
    pub finite_only: bool,
    /// Normalize `-0.0` to `+0.0` before fingerprinting.
    pub collapse_negative_zero: bool,
    /// Float serialization representation used for fingerprints.
    pub representation: CanonicalFloatRepresentation,
}

impl Default for CanonicalFloatRules {
    fn default() -> Self {
        Self {
            finite_only: true,
            collapse_negative_zero: true,
            representation: CanonicalFloatRepresentation::Ieee754Binary64Hex,
        }
    }
}

/// Canonical float representation used by stage fingerprints.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalFloatRepresentation {
    /// Lowercase hexadecimal IEEE-754 binary64 bits after normalization.
    Ieee754Binary64Hex,
}

/// Deterministic operation ordering contract.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationOrderingRule {
    /// `ModelingProgram.operations` is the exact replay order. Operation IDs
    /// must be unique, dependency edges must be canonical, and producers must
    /// appear before consumers.
    ProgramOrderStableIdsProducerBeforeConsumer,
}

/// Stage fingerprint inclusion policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageFingerprintPolicy {
    /// Include the canonical evaluator config in the input stage.
    pub include_evaluator_config: bool,
    /// Include canonical program inputs and each ordered operation.
    pub include_semantic_inputs: bool,
    /// Exclude adapter caches from semantic stage fingerprints.
    pub exclude_adapter_caches: bool,
}

impl Default for StageFingerprintPolicy {
    fn default() -> Self {
        Self {
            include_evaluator_config: true,
            include_semantic_inputs: true,
            exclude_adapter_caches: true,
        }
    }
}

/// Replay trace policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayTracePolicy {
    /// Record each operation's exact ordinal.
    pub record_operation_ordinals: bool,
    /// Record pre- and post-operation semantic stage fingerprints.
    pub record_stage_fingerprints: bool,
}

impl Default for ReplayTracePolicy {
    fn default() -> Self {
        Self {
            record_operation_ordinals: true,
            record_stage_fingerprints: true,
        }
    }
}

/// Adapter cache policy. Adapter caches may speed host integration but may not
/// change semantic results.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCachePolicy {
    /// Adapter caches are allowed to be reported.
    pub enabled: bool,
    /// Adapter cache fingerprints are recorded separately from semantic stages.
    pub record_cache_fingerprints: bool,
    /// Semantic result fingerprints exclude adapter cache fingerprints.
    pub exclude_from_semantic_result: bool,
}

impl Default for AdapterCachePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            record_cache_fingerprints: true,
            exclude_from_semantic_result: true,
        }
    }
}

/// Kind of semantic stage represented by a fingerprint.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationStageKind {
    /// Canonical program input before the first operation.
    InputProgram,
    /// One ordered semantic operation.
    Operation,
    /// Final semantic result state.
    SemanticResult,
}

/// Deterministic semantic stage fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageFingerprint {
    /// Stage kind.
    pub stage: EvaluationStageKind,
    /// Ordered operation ordinal when this is an operation stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_index: Option<usize>,
    /// Stable operation ID when this is an operation stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<ProgramOperationId>,
    /// Deterministic fingerprint for the semantic stage.
    pub fingerprint: String,
}

/// One operation step in an exact replay trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayTraceOperation {
    /// Exact replay ordinal from `ModelingProgram.operations`.
    pub operation_index: usize,
    /// Stable operation ID.
    pub operation_id: ProgramOperationId,
    /// Operation vocabulary kind.
    pub operation_kind: ModelingOperationKind,
    /// Semantic stage fingerprint before this operation.
    pub input_stage_fingerprint: String,
    /// Semantic stage fingerprint after this operation.
    pub output_stage_fingerprint: String,
}

/// Serializable replay trace for exact deterministic evaluator runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayTrace {
    /// Evaluator config used to build the trace.
    pub config: EvaluatorConfig,
    /// Ordered replay operations.
    pub operations: Vec<ReplayTraceOperation>,
    /// Input, operation, and final semantic stage fingerprints.
    pub stage_fingerprints: Vec<StageFingerprint>,
    /// Deterministic fingerprint of the replay trace itself.
    pub trace_fingerprint: String,
}

/// Semantic result summary. Adapter cache reports are intentionally excluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorSemanticResult {
    /// Evaluator config used for the result.
    pub config: EvaluatorConfig,
    /// Exact replay trace.
    pub replay_trace: ReplayTrace,
    /// Final semantic stage fingerprint.
    pub final_stage_fingerprint: StageFingerprint,
}

impl EvaluatorSemanticResult {
    /// Build a semantic result from a replay trace.
    #[must_use]
    pub fn from_replay_trace(replay_trace: ReplayTrace) -> Self {
        let final_stage_fingerprint = replay_trace
            .stage_fingerprints
            .last()
            .cloned()
            .unwrap_or_else(|| StageFingerprint {
                stage: EvaluationStageKind::SemanticResult,
                operation_index: None,
                operation_id: None,
                fingerprint: String::new(),
            });

        Self {
            config: replay_trace.config.clone(),
            replay_trace,
            final_stage_fingerprint,
        }
    }

    /// Fingerprint semantic result data only. Adapter caches are not part of
    /// this input and cannot change the returned value.
    pub fn semantic_fingerprint(&self) -> Result<String, EvaluatorContractError> {
        fingerprint_serializable(SEMANTIC_RESULT_FINGERPRINT_DOMAIN, self)
    }
}

/// Blender adapter cache report kept outside semantic evaluator results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlenderAdapterCacheReport {
    /// Adapter implementation version or build label.
    pub adapter_version: String,
    /// Cache entries produced or consumed by the adapter.
    pub entries: Vec<BlenderAdapterCacheEntry>,
}

impl BlenderAdapterCacheReport {
    /// Fingerprint adapter cache data separately from semantic results.
    pub fn cache_fingerprint(&self) -> Result<String, EvaluatorContractError> {
        fingerprint_serializable(ADAPTER_CACHE_FINGERPRINT_DOMAIN, self)
    }
}

/// One Blender adapter cache entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlenderAdapterCacheEntry {
    /// Stable cache key inside the adapter.
    pub key: String,
    /// Semantic stage this cache was derived from.
    pub source_stage_fingerprint: String,
    /// Fingerprint of cache bytes or host-specific serialized cache metadata.
    pub cache_fingerprint: String,
    /// Cache payload byte count.
    pub byte_len: usize,
}

/// Full evaluator output envelope with adapter caches separated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorOutputEnvelope {
    /// Semantic result, independent of adapter caches.
    pub semantic_result: EvaluatorSemanticResult,
    /// Optional host adapter cache report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blender_adapter_cache: Option<BlenderAdapterCacheReport>,
}

impl EvaluatorOutputEnvelope {
    /// Fingerprint semantic result data while excluding adapter caches.
    pub fn semantic_result_fingerprint(&self) -> Result<String, EvaluatorContractError> {
        self.semantic_result.semantic_fingerprint()
    }

    /// Fingerprint adapter cache data separately when present.
    pub fn adapter_cache_fingerprint(&self) -> Result<Option<String>, EvaluatorContractError> {
        self.blender_adapter_cache
            .as_ref()
            .map(BlenderAdapterCacheReport::cache_fingerprint)
            .transpose()
    }
}

/// Platform determinism probe for contract-level diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformDeterminismReport {
    /// Evaluator config used for the probe.
    pub config: EvaluatorConfig,
    /// Canonical `+0.0` bits after normalization.
    pub positive_zero_bits: String,
    /// Canonical `-0.0` bits after normalization.
    pub negative_zero_bits: String,
    /// Canonical `1.5` bits.
    pub one_point_five_bits: String,
    /// Fingerprint of this normalized probe.
    pub probe_fingerprint: String,
}

/// Build a deterministic platform probe from canonical float rules.
pub fn platform_determinism_report(
    config: &EvaluatorConfig,
) -> Result<PlatformDeterminismReport, EvaluatorContractError> {
    let mut report = PlatformDeterminismReport {
        config: config.clone(),
        positive_zero_bits: canonical_f64_hex(0.0, config.float_rules, "probe.positive_zero")?,
        negative_zero_bits: canonical_f64_hex(-0.0, config.float_rules, "probe.negative_zero")?,
        one_point_five_bits: canonical_f64_hex(1.5, config.float_rules, "probe.one_point_five")?,
        probe_fingerprint: String::new(),
    };
    report.probe_fingerprint =
        fingerprint_serializable("shape-program.evaluator.platform-probe.v1", &report)?;
    Ok(report)
}

/// Validate that a program's declared operation order is deterministic and
/// dependency-compatible.
pub fn validate_deterministic_operation_order(
    program: &ModelingProgram,
) -> Result<(), EvaluatorContractError> {
    let mut operation_ordinals = BTreeMap::new();
    for (ordinal, operation) in program.operations.iter().enumerate() {
        if operation_ordinals
            .insert(operation.id.0.clone(), ordinal)
            .is_some()
        {
            return Err(EvaluatorContractError::DuplicateOperationId {
                operation_id: operation.id.0.clone(),
            });
        }
    }

    let mut selection_ids = BTreeSet::new();
    for selection in &program.selections {
        if !selection_ids.insert(selection.id.0.clone()) {
            return Err(EvaluatorContractError::DuplicateSelectionId {
                selection_id: selection.id.0.clone(),
            });
        }
    }

    for operation in &program.operations {
        for selection_id in &operation.selections {
            if !selection_ids.contains(&selection_id.0) {
                return Err(EvaluatorContractError::UnknownSelectionReference {
                    operation_id: operation.id.0.clone(),
                    selection_id: selection_id.0.clone(),
                });
            }
        }
    }

    ensure_sorted_unique_operation_edges(&program.dependency_graph.operation_edges)?;
    ensure_sorted_unique_selection_edges(&program.dependency_graph.selection_edges)?;

    for (producer, consumer) in &program.dependency_graph.operation_edges {
        let producer_ordinal = operation_ordinals.get(&producer.0).ok_or_else(|| {
            EvaluatorContractError::UnknownOperationReference {
                operation_id: producer.0.clone(),
            }
        })?;
        let consumer_ordinal = operation_ordinals.get(&consumer.0).ok_or_else(|| {
            EvaluatorContractError::UnknownOperationReference {
                operation_id: consumer.0.clone(),
            }
        })?;
        if producer_ordinal >= consumer_ordinal {
            return Err(EvaluatorContractError::ProducerAfterConsumer {
                producer_id: producer.0.clone(),
                consumer_id: consumer.0.clone(),
            });
        }
    }

    for (selection, operation) in &program.dependency_graph.selection_edges {
        if !selection_ids.contains(&selection.0) {
            return Err(EvaluatorContractError::UnknownSelectionReference {
                operation_id: operation.0.clone(),
                selection_id: selection.0.clone(),
            });
        }
        if !operation_ordinals.contains_key(&operation.0) {
            return Err(EvaluatorContractError::UnknownOperationReference {
                operation_id: operation.0.clone(),
            });
        }
    }

    Ok(())
}

/// Build a deterministic replay trace without evaluating mesh geometry.
pub fn build_replay_trace(
    program: &ModelingProgram,
    config: &EvaluatorConfig,
) -> Result<ReplayTrace, EvaluatorContractError> {
    validate_evaluator_contract(program, config)?;
    validate_deterministic_operation_order(program)?;

    let canonical_program = canonical_program(program, config.float_rules)?;
    let input_fingerprint = fingerprint_stage(&StageFingerprintInput {
        stage: EvaluationStageKind::InputProgram,
        operation_index: None,
        operation: None,
        prior_stage_fingerprint: None,
        config: config.clone(),
        program: Some(canonical_program.clone()),
    })?;

    let mut stage_fingerprints = vec![StageFingerprint {
        stage: EvaluationStageKind::InputProgram,
        operation_index: None,
        operation_id: None,
        fingerprint: input_fingerprint,
    }];
    let mut operations = Vec::with_capacity(canonical_program.operations.len());

    for (operation_index, operation) in canonical_program.operations.iter().enumerate() {
        let input_stage_fingerprint = stage_fingerprints
            .last()
            .map(|stage| stage.fingerprint.clone())
            .unwrap_or_default();
        let output_stage_fingerprint = fingerprint_stage(&StageFingerprintInput {
            stage: EvaluationStageKind::Operation,
            operation_index: Some(operation_index),
            operation: Some(operation.clone()),
            prior_stage_fingerprint: Some(input_stage_fingerprint.clone()),
            config: config.clone(),
            program: None,
        })?;

        stage_fingerprints.push(StageFingerprint {
            stage: EvaluationStageKind::Operation,
            operation_index: Some(operation_index),
            operation_id: Some(operation.id.clone()),
            fingerprint: output_stage_fingerprint.clone(),
        });
        operations.push(ReplayTraceOperation {
            operation_index,
            operation_id: operation.id.clone(),
            operation_kind: operation.kind,
            input_stage_fingerprint,
            output_stage_fingerprint,
        });
    }

    let prior_stage_fingerprint = stage_fingerprints
        .last()
        .map(|stage| stage.fingerprint.clone())
        .unwrap_or_default();
    let final_fingerprint = fingerprint_stage(&StageFingerprintInput {
        stage: EvaluationStageKind::SemanticResult,
        operation_index: None,
        operation: None,
        prior_stage_fingerprint: Some(prior_stage_fingerprint),
        config: config.clone(),
        program: None,
    })?;
    stage_fingerprints.push(StageFingerprint {
        stage: EvaluationStageKind::SemanticResult,
        operation_index: None,
        operation_id: None,
        fingerprint: final_fingerprint,
    });

    let trace_fingerprint = fingerprint_serializable(
        TRACE_FINGERPRINT_DOMAIN,
        &ReplayTraceFingerprintInput {
            config: config.clone(),
            operations: operations.clone(),
            stage_fingerprints: stage_fingerprints.clone(),
        },
    )?;

    Ok(ReplayTrace {
        config: config.clone(),
        operations,
        stage_fingerprints,
        trace_fingerprint,
    })
}

/// Fingerprint canonical semantic output independent of admissible program-order
/// differences. This is not a replay trace fingerprint; it intentionally omits
/// operation ordinals and dependency edges so commuted equivalent histories can
/// still declare the same output.
pub fn semantic_output_fingerprint(
    program: &ModelingProgram,
    config: &EvaluatorConfig,
) -> Result<String, EvaluatorContractError> {
    validate_evaluator_contract(program, config)?;
    validate_deterministic_operation_order(program)?;

    let mut operations = program
        .operations
        .iter()
        .enumerate()
        .map(|(operation_index, operation)| {
            canonical_operation(operation_index, operation, config.float_rules)
        })
        .collect::<Result<Vec<_>, _>>()?;
    operations.sort_by(|left, right| left.id.cmp(&right.id));

    let mut selections = program
        .selections
        .iter()
        .map(|selection| canonical_selection(selection, config.float_rules))
        .collect::<Result<Vec<_>, _>>()?;
    selections.sort_by(|left, right| left.id.cmp(&right.id));

    fingerprint_serializable(
        SEMANTIC_OUTPUT_FINGERPRINT_DOMAIN,
        &SemanticOutputFingerprintInput {
            config: config.clone(),
            schema_version: program.schema_version,
            grammar_profile: program.grammar_profile,
            base_topology: program.base_topology.clone(),
            operations,
            selections,
            canonical_evaluator_version: program.canonical_evaluator_version.clone(),
        },
    )
}

/// Evaluator contract validation and fingerprint errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvaluatorContractError {
    /// Program evaluator version and config evaluator version differ.
    #[error("program requires evaluator {program_version}, but config uses {config_version}")]
    EvaluatorVersionMismatch {
        /// Version declared by the program.
        program_version: String,
        /// Version declared by the config.
        config_version: String,
    },
    /// Unsupported evaluator config schema version.
    #[error("unsupported evaluator config schema version {schema_version}")]
    UnsupportedConfigSchemaVersion {
        /// Unsupported schema version.
        schema_version: u32,
    },
    /// Unsupported modeling program schema version.
    #[error("unsupported modeling program schema version {schema_version}")]
    UnsupportedProgramSchemaVersion {
        /// Unsupported schema version.
        schema_version: u32,
    },
    /// Duplicate operation ID.
    #[error("duplicate operation ID {operation_id}")]
    DuplicateOperationId {
        /// Duplicated operation ID.
        operation_id: String,
    },
    /// Duplicate selection ID.
    #[error("duplicate selection ID {selection_id}")]
    DuplicateSelectionId {
        /// Duplicated selection ID.
        selection_id: String,
    },
    /// Dependency edge references an unknown operation.
    #[error("unknown operation reference {operation_id}")]
    UnknownOperationReference {
        /// Unknown operation ID.
        operation_id: String,
    },
    /// Operation or dependency edge references an unknown selection.
    #[error("operation {operation_id} references unknown selection {selection_id}")]
    UnknownSelectionReference {
        /// Operation ID.
        operation_id: String,
        /// Unknown selection ID.
        selection_id: String,
    },
    /// Dependency producer appears after its consumer in exact replay order.
    #[error("dependency producer {producer_id} must appear before consumer {consumer_id}")]
    ProducerAfterConsumer {
        /// Producer operation ID.
        producer_id: String,
        /// Consumer operation ID.
        consumer_id: String,
    },
    /// Dependency edges are not in canonical sorted unique order.
    #[error("{edge_kind} dependency edges must be sorted and unique")]
    NonCanonicalDependencyEdgeOrder {
        /// Edge set kind.
        edge_kind: &'static str,
    },
    /// Non-finite float encountered under finite-only canonical rules.
    #[error("non-finite float at {subject}")]
    NonFiniteFloat {
        /// Subject path.
        subject: String,
    },
    /// Serialization failed while building a deterministic fingerprint.
    #[error("failed to serialize {subject} for deterministic fingerprint: {error}")]
    Serialization {
        /// Serialized subject.
        subject: &'static str,
        /// Serialization error.
        error: String,
    },
}

fn validate_evaluator_contract(
    program: &ModelingProgram,
    config: &EvaluatorConfig,
) -> Result<(), EvaluatorContractError> {
    if config.schema_version != EVALUATOR_CONTRACT_SCHEMA_VERSION {
        return Err(EvaluatorContractError::UnsupportedConfigSchemaVersion {
            schema_version: config.schema_version,
        });
    }
    if program.schema_version != MODELING_PROGRAM_SCHEMA_VERSION {
        return Err(EvaluatorContractError::UnsupportedProgramSchemaVersion {
            schema_version: program.schema_version,
        });
    }
    if program.canonical_evaluator_version != config.evaluator_version {
        return Err(EvaluatorContractError::EvaluatorVersionMismatch {
            program_version: program.canonical_evaluator_version.clone(),
            config_version: config.evaluator_version.clone(),
        });
    }
    Ok(())
}

fn ensure_sorted_unique_operation_edges(
    edges: &[(ProgramOperationId, ProgramOperationId)],
) -> Result<(), EvaluatorContractError> {
    if edges.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(EvaluatorContractError::NonCanonicalDependencyEdgeOrder {
            edge_kind: "operation",
        });
    }
    Ok(())
}

fn ensure_sorted_unique_selection_edges(
    edges: &[(SemanticSelectionId, ProgramOperationId)],
) -> Result<(), EvaluatorContractError> {
    if edges.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(EvaluatorContractError::NonCanonicalDependencyEdgeOrder {
            edge_kind: "selection",
        });
    }
    Ok(())
}

fn fingerprint_stage(input: &StageFingerprintInput) -> Result<String, EvaluatorContractError> {
    fingerprint_serializable(STAGE_FINGERPRINT_DOMAIN, input)
}

fn fingerprint_serializable<T: Serialize>(
    domain: &'static str,
    value: &T,
) -> Result<String, EvaluatorContractError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| EvaluatorContractError::Serialization {
            subject: domain,
            error: error.to_string(),
        })?;
    Ok(deterministic_fingerprint_bytes(domain, &bytes))
}

fn deterministic_fingerprint_bytes(domain: &str, bytes: &[u8]) -> String {
    const FNV_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const FNV_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

    let mut hash = FNV_OFFSET;
    for byte in domain
        .as_bytes()
        .iter()
        .copied()
        .chain([0xff])
        .chain(bytes.iter().copied())
    {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:032x}")
}

fn canonical_f64_hex(
    value: f64,
    rules: CanonicalFloatRules,
    subject: impl Into<String>,
) -> Result<String, EvaluatorContractError> {
    if rules.finite_only && !value.is_finite() {
        return Err(EvaluatorContractError::NonFiniteFloat {
            subject: subject.into(),
        });
    }

    let normalized = if rules.collapse_negative_zero && value == 0.0 {
        0.0
    } else {
        value
    };

    Ok(format!("{:016x}", normalized.to_bits()))
}

fn canonical_program(
    program: &ModelingProgram,
    rules: CanonicalFloatRules,
) -> Result<CanonicalProgram, EvaluatorContractError> {
    Ok(CanonicalProgram {
        schema_version: program.schema_version,
        grammar_profile: program.grammar_profile,
        base_topology: program.base_topology.clone(),
        operations: program
            .operations
            .iter()
            .enumerate()
            .map(|(operation_index, operation)| {
                canonical_operation(operation_index, operation, rules)
            })
            .collect::<Result<_, _>>()?,
        selections: program
            .selections
            .iter()
            .map(|selection| canonical_selection(selection, rules))
            .collect::<Result<_, _>>()?,
        dependency_graph: program.dependency_graph.clone(),
        canonical_evaluator_version: program.canonical_evaluator_version.clone(),
    })
}

fn canonical_operation(
    operation_index: usize,
    operation: &ModelingOperation,
    rules: CanonicalFloatRules,
) -> Result<CanonicalModelingOperation, EvaluatorContractError> {
    Ok(CanonicalModelingOperation {
        id: operation.id.clone(),
        kind: operation.kind,
        selections: operation.selections.clone(),
        parameters: operation
            .parameters
            .iter()
            .enumerate()
            .map(|(parameter_index, parameter)| {
                canonical_parameter(operation_index, parameter_index, parameter, rules)
            })
            .collect::<Result<_, _>>()?,
        affected_element_count: operation.affected_element_count,
        payloads: operation.payloads.clone(),
    })
}

fn canonical_parameter(
    operation_index: usize,
    parameter_index: usize,
    parameter: &SemanticParameter,
    rules: CanonicalFloatRules,
) -> Result<CanonicalSemanticParameter, EvaluatorContractError> {
    let subject = |field: &str| {
        format!("operations[{operation_index}].parameters[{parameter_index}].{field}")
    };
    Ok(match parameter {
        SemanticParameter::Scalar { name, value } => CanonicalSemanticParameter::Scalar {
            name: name.clone(),
            value: canonical_f64_hex(*value, rules, subject("value"))?,
        },
        SemanticParameter::Integer { name, value } => CanonicalSemanticParameter::Integer {
            name: name.clone(),
            value: *value,
        },
        SemanticParameter::Boolean { name, value } => CanonicalSemanticParameter::Boolean {
            name: name.clone(),
            value: *value,
        },
        SemanticParameter::Choice { name, value } => CanonicalSemanticParameter::Choice {
            name: name.clone(),
            value: value.clone(),
        },
        SemanticParameter::Vector3 { name, value } => CanonicalSemanticParameter::Vector3 {
            name: name.clone(),
            value: [
                canonical_f64_hex(value[0], rules, subject("value[0]"))?,
                canonical_f64_hex(value[1], rules, subject("value[1]"))?,
                canonical_f64_hex(value[2], rules, subject("value[2]"))?,
            ],
        },
        SemanticParameter::Quaternion { name, value } => CanonicalSemanticParameter::Quaternion {
            name: name.clone(),
            value: [
                canonical_f64_hex(value[0], rules, subject("value[0]"))?,
                canonical_f64_hex(value[1], rules, subject("value[1]"))?,
                canonical_f64_hex(value[2], rules, subject("value[2]"))?,
                canonical_f64_hex(value[3], rules, subject("value[3]"))?,
            ],
        },
    })
}

fn canonical_selection(
    selection: &SemanticSelection,
    rules: CanonicalFloatRules,
) -> Result<CanonicalSemanticSelection, EvaluatorContractError> {
    Ok(CanonicalSemanticSelection {
        id: selection.id.clone(),
        payload: canonical_selection_payload(&selection.id, &selection.payload, rules)?,
    })
}

fn canonical_selection_payload(
    selection_id: &SemanticSelectionId,
    payload: &SemanticSelectionPayload,
    rules: CanonicalFloatRules,
) -> Result<CanonicalSemanticSelectionPayload, EvaluatorContractError> {
    let subject = |field: &str| format!("selections.{}.{}", selection_id.0, field);
    Ok(match payload {
        SemanticSelectionPayload::Part { part } => {
            CanonicalSemanticSelectionPayload::Part { part: part.clone() }
        }
        SemanticSelectionPayload::Region { region } => CanonicalSemanticSelectionPayload::Region {
            region: region.clone(),
        },
        SemanticSelectionPayload::BoundaryLoop { boundary_loop } => {
            CanonicalSemanticSelectionPayload::BoundaryLoop {
                boundary_loop: boundary_loop.clone(),
            }
        }
        SemanticSelectionPayload::EdgeClass { class } => {
            CanonicalSemanticSelectionPayload::EdgeClass {
                class: class.clone(),
            }
        }
        SemanticSelectionPayload::FacePatch { patch } => {
            CanonicalSemanticSelectionPayload::FacePatch {
                patch: patch.clone(),
            }
        }
        SemanticSelectionPayload::SymmetryPartner { selection } => {
            CanonicalSemanticSelectionPayload::SymmetryPartner {
                selection: selection.clone(),
            }
        }
        SemanticSelectionPayload::GeodesicNeighborhood { seed, radius } => {
            CanonicalSemanticSelectionPayload::GeodesicNeighborhood {
                seed: seed.clone(),
                radius: canonical_f64_hex(*radius, rules, subject("radius"))?,
            }
        }
        SemanticSelectionPayload::SpatialPrimitive { shape } => {
            CanonicalSemanticSelectionPayload::SpatialPrimitive {
                shape: canonical_spatial_primitive(selection_id, shape, rules)?,
            }
        }
        SemanticSelectionPayload::BooleanOperand { operand_id } => {
            CanonicalSemanticSelectionPayload::BooleanOperand {
                operand_id: operand_id.clone(),
            }
        }
        SemanticSelectionPayload::CompactFalloffField {
            field_id,
            parameter_count,
        } => CanonicalSemanticSelectionPayload::CompactFalloffField {
            field_id: field_id.clone(),
            parameter_count: *parameter_count,
        },
        SemanticSelectionPayload::SemanticLandmarkGroup { group_id } => {
            CanonicalSemanticSelectionPayload::SemanticLandmarkGroup {
                group_id: group_id.clone(),
            }
        }
        SemanticSelectionPayload::ExplicitIndices { target, indices } => {
            CanonicalSemanticSelectionPayload::ExplicitIndices {
                target: *target,
                indices: indices.clone(),
            }
        }
    })
}

fn canonical_spatial_primitive(
    selection_id: &SemanticSelectionId,
    shape: &SpatialPrimitiveSelection,
    rules: CanonicalFloatRules,
) -> Result<CanonicalSpatialPrimitiveSelection, EvaluatorContractError> {
    let subject = |field: &str| format!("selections.{}.shape.{}", selection_id.0, field);
    Ok(match shape {
        SpatialPrimitiveSelection::Sphere { center, radius } => {
            CanonicalSpatialPrimitiveSelection::Sphere {
                center: [
                    canonical_f64_hex(center[0], rules, subject("center[0]"))?,
                    canonical_f64_hex(center[1], rules, subject("center[1]"))?,
                    canonical_f64_hex(center[2], rules, subject("center[2]"))?,
                ],
                radius: canonical_f64_hex(*radius, rules, subject("radius"))?,
            }
        }
        SpatialPrimitiveSelection::Box { min, max } => CanonicalSpatialPrimitiveSelection::Box {
            min: [
                canonical_f64_hex(min[0], rules, subject("min[0]"))?,
                canonical_f64_hex(min[1], rules, subject("min[1]"))?,
                canonical_f64_hex(min[2], rules, subject("min[2]"))?,
            ],
            max: [
                canonical_f64_hex(max[0], rules, subject("max[0]"))?,
                canonical_f64_hex(max[1], rules, subject("max[1]"))?,
                canonical_f64_hex(max[2], rules, subject("max[2]"))?,
            ],
        },
        SpatialPrimitiveSelection::PlaneSlab {
            normal,
            offset,
            half_width,
        } => CanonicalSpatialPrimitiveSelection::PlaneSlab {
            normal: [
                canonical_f64_hex(normal[0], rules, subject("normal[0]"))?,
                canonical_f64_hex(normal[1], rules, subject("normal[1]"))?,
                canonical_f64_hex(normal[2], rules, subject("normal[2]"))?,
            ],
            offset: canonical_f64_hex(*offset, rules, subject("offset"))?,
            half_width: canonical_f64_hex(*half_width, rules, subject("half_width"))?,
        },
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ReplayTraceFingerprintInput {
    config: EvaluatorConfig,
    operations: Vec<ReplayTraceOperation>,
    stage_fingerprints: Vec<StageFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SemanticOutputFingerprintInput {
    config: EvaluatorConfig,
    schema_version: u32,
    grammar_profile: GrammarProfile,
    base_topology: Option<BaseTopologyReference>,
    operations: Vec<CanonicalModelingOperation>,
    selections: Vec<CanonicalSemanticSelection>,
    canonical_evaluator_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StageFingerprintInput {
    stage: EvaluationStageKind,
    operation_index: Option<usize>,
    operation: Option<CanonicalModelingOperation>,
    prior_stage_fingerprint: Option<String>,
    config: EvaluatorConfig,
    program: Option<CanonicalProgram>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CanonicalProgram {
    schema_version: u32,
    grammar_profile: GrammarProfile,
    base_topology: Option<BaseTopologyReference>,
    operations: Vec<CanonicalModelingOperation>,
    selections: Vec<CanonicalSemanticSelection>,
    dependency_graph: ProgramDependencyGraph,
    canonical_evaluator_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CanonicalModelingOperation {
    id: ProgramOperationId,
    kind: ModelingOperationKind,
    selections: Vec<SemanticSelectionId>,
    parameters: Vec<CanonicalSemanticParameter>,
    affected_element_count: usize,
    payloads: Vec<OperationPayloadDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalSemanticParameter {
    Scalar { name: String, value: String },
    Integer { name: String, value: i64 },
    Boolean { name: String, value: bool },
    Choice { name: String, value: String },
    Vector3 { name: String, value: [String; 3] },
    Quaternion { name: String, value: [String; 4] },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CanonicalSemanticSelection {
    id: SemanticSelectionId,
    payload: CanonicalSemanticSelectionPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalSemanticSelectionPayload {
    Part {
        part: SemanticPartId,
    },
    Region {
        region: SemanticRegionId,
    },
    BoundaryLoop {
        boundary_loop: SemanticBoundaryLoopId,
    },
    EdgeClass {
        class: String,
    },
    FacePatch {
        patch: String,
    },
    SymmetryPartner {
        selection: SemanticSelectionId,
    },
    GeodesicNeighborhood {
        seed: SemanticSelectionId,
        radius: String,
    },
    SpatialPrimitive {
        shape: CanonicalSpatialPrimitiveSelection,
    },
    BooleanOperand {
        operand_id: String,
    },
    CompactFalloffField {
        field_id: String,
        parameter_count: usize,
    },
    SemanticLandmarkGroup {
        group_id: String,
    },
    ExplicitIndices {
        target: ExplicitSelectionTarget,
        indices: Vec<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
enum CanonicalSpatialPrimitiveSelection {
    Sphere {
        center: [String; 3],
        radius: String,
    },
    Box {
        min: [String; 3],
        max: [String; 3],
    },
    PlaneSlab {
        normal: [String; 3],
        offset: String,
        half_width: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModelingOperation, OperationPayloadKind};

    #[test]
    fn replay_trace_ordering_is_stable() {
        let program = fixture_program(0.25);
        let config = EvaluatorConfig::canonical();

        let first = build_replay_trace(&program, &config).expect("trace builds");
        let second = build_replay_trace(&program, &config).expect("trace builds again");

        assert_eq!(first, second);
        assert_eq!(
            first
                .operations
                .iter()
                .map(|operation| operation.operation_id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["op.001.create", "op.002.extrude"]
        );

        let json = serde_json::to_string(&first).expect("trace serializes");
        let round_trip: ReplayTrace = serde_json::from_str(&json).expect("trace deserializes");
        assert_eq!(round_trip, first);
    }

    #[test]
    fn adapter_cache_fingerprints_are_excluded_from_semantic_result_fingerprints() {
        let program = fixture_program(0.25);
        let config = EvaluatorConfig::canonical();
        let trace = build_replay_trace(&program, &config).expect("trace builds");
        let semantic_result = EvaluatorSemanticResult::from_replay_trace(trace);

        let first = EvaluatorOutputEnvelope {
            semantic_result: semantic_result.clone(),
            blender_adapter_cache: Some(cache_report("adapter-cache-a", "aaaaaaaa")),
        };
        let second = EvaluatorOutputEnvelope {
            semantic_result,
            blender_adapter_cache: Some(cache_report("adapter-cache-b", "bbbbbbbb")),
        };

        assert_eq!(
            first
                .semantic_result_fingerprint()
                .expect("semantic fingerprint"),
            second
                .semantic_result_fingerprint()
                .expect("semantic fingerprint")
        );
        assert_ne!(
            first
                .adapter_cache_fingerprint()
                .expect("adapter cache fingerprint"),
            second
                .adapter_cache_fingerprint()
                .expect("adapter cache fingerprint")
        );
    }

    #[test]
    fn deterministic_ordering_rejects_producer_after_consumer() {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation::compact(
            "op.consumer",
            ModelingOperationKind::RegionExtrude,
        ));
        program.operations.push(ModelingOperation::compact(
            "op.producer",
            ModelingOperationKind::PrimitiveCreate,
        ));
        program.dependency_graph.operation_edges.push((
            ProgramOperationId("op.producer".to_owned()),
            ProgramOperationId("op.consumer".to_owned()),
        ));

        let error = validate_deterministic_operation_order(&program)
            .expect_err("producer after consumer should fail");

        assert_eq!(
            error,
            EvaluatorContractError::ProducerAfterConsumer {
                producer_id: "op.producer".to_owned(),
                consumer_id: "op.consumer".to_owned()
            }
        );
    }

    #[test]
    fn canonical_float_rules_are_platform_stable() {
        let config = EvaluatorConfig::canonical();
        let report = platform_determinism_report(&config).expect("probe builds");

        assert_eq!(report.positive_zero_bits, "0000000000000000");
        assert_eq!(report.negative_zero_bits, "0000000000000000");
        assert_eq!(report.one_point_five_bits, "3ff8000000000000");

        let positive_zero = fixture_program(0.0);
        let negative_zero = fixture_program(-0.0);

        let positive_result = EvaluatorSemanticResult::from_replay_trace(
            build_replay_trace(&positive_zero, &config).expect("positive-zero trace"),
        );
        let negative_result = EvaluatorSemanticResult::from_replay_trace(
            build_replay_trace(&negative_zero, &config).expect("negative-zero trace"),
        );

        assert_eq!(
            positive_result
                .semantic_fingerprint()
                .expect("positive fingerprint"),
            negative_result
                .semantic_fingerprint()
                .expect("negative fingerprint")
        );
    }

    #[test]
    fn non_finite_float_is_rejected() {
        let mut program = fixture_program(f64::NAN);
        program.operations[0]
            .parameters
            .push(SemanticParameter::Scalar {
                name: "bad".to_owned(),
                value: f64::INFINITY,
            });

        let error = build_replay_trace(&program, &EvaluatorConfig::canonical())
            .expect_err("non-finite float should fail");

        assert!(matches!(
            error,
            EvaluatorContractError::NonFiniteFloat { .. }
        ));
    }

    fn fixture_program(offset: f64) -> ModelingProgram {
        let mut program = ModelingProgram::strict_from_primitives();
        program.operations.push(ModelingOperation {
            id: ProgramOperationId("op.001.create".to_owned()),
            kind: ModelingOperationKind::PrimitiveCreate,
            selections: Vec::new(),
            parameters: vec![SemanticParameter::Vector3 {
                name: "center".to_owned(),
                value: [offset, 1.0, 2.0],
            }],
            affected_element_count: 1,
            payloads: vec![OperationPayloadDescriptor {
                kind: OperationPayloadKind::SemanticParameters,
                encoded_bytes: 24,
                semantic_parameter_count: 3,
                affected_element_count: 1,
                perturbation_valid: true,
            }],
        });
        program.operations.push(ModelingOperation {
            id: ProgramOperationId("op.002.extrude".to_owned()),
            kind: ModelingOperationKind::RegionExtrude,
            selections: Vec::new(),
            parameters: vec![SemanticParameter::Scalar {
                name: "distance".to_owned(),
                value: 0.5,
            }],
            affected_element_count: 4,
            payloads: vec![OperationPayloadDescriptor {
                kind: OperationPayloadKind::SemanticParameters,
                encoded_bytes: 8,
                semantic_parameter_count: 1,
                affected_element_count: 4,
                perturbation_valid: true,
            }],
        });
        program.dependency_graph.operation_edges.push((
            ProgramOperationId("op.001.create".to_owned()),
            ProgramOperationId("op.002.extrude".to_owned()),
        ));
        program
    }

    fn cache_report(key: &str, cache_fingerprint: &str) -> BlenderAdapterCacheReport {
        BlenderAdapterCacheReport {
            adapter_version: "blender-adapter-test".to_owned(),
            entries: vec![BlenderAdapterCacheEntry {
                key: key.to_owned(),
                source_stage_fingerprint: "semantic-stage".to_owned(),
                cache_fingerprint: cache_fingerprint.to_owned(),
                byte_len: 64,
            }],
        }
    }
}
