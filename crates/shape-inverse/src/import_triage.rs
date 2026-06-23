//! Product-facing import triage reports.
//!
//! This module composes existing diagnostic and strict-recovery gates into a
//! truthful import workflow. It does not claim editable import unless strict
//! verification succeeds.

use serde::{Deserialize, Serialize};
use shape_character::corpus::{CharacterMeshArtifact, character_mesh_artifact_fingerprint};

use crate::{
    analysis::Confidence,
    character_recovery::{
        KnownBaseCharacterRecoveryCaseReport, recover_known_base_character_mesh_artifact,
    },
    external_character::{
        EXTERNAL_CHARACTER_ANALYSIS_SCHEMA_VERSION, ExternalCharacterAnalysisConfig,
        ExternalCharacterAnalysisReport, ExternalCharacterCanonicalization,
        ExternalCharacterImportOutcome, ExternalCharacterIssue, ExternalCharacterIssueSeverity,
        ExternalCharacterRejectionReason, ExternalCleanCharacterMeshInput,
        analyze_external_clean_character_mesh,
    },
};

/// Current schema for import triage reports.
pub const IMPORT_TRIAGE_SCHEMA_VERSION: u32 = 1;

/// Full product-facing triage report for one import attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportTriageReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Stable source identifier copied from the input.
    pub source_id: String,
    /// Triage source kind.
    pub source_kind: ImportTriageSourceKind,
    /// Product-facing outcome.
    pub outcome: ImportTriageOutcome,
    /// Approved user-facing label for this outcome.
    pub user_facing_label: String,
    /// True only when a strict proof has accepted.
    pub strict_recovery_proven: bool,
    /// Strict recovery confidence. This is `1.0` only after strict proof.
    pub strict_recovery_confidence: Confidence,
    /// Best classification confidence from the diagnostic analysis.
    pub classification_confidence: Confidence,
    /// External clean-character analysis used by this triage report.
    pub external_character_analysis: ExternalCharacterAnalysisReport,
    /// Strict known-base recovery proof, when the descriptor was eligible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_known_base_recovery: Option<KnownBaseCharacterRecoveryCaseReport>,
    /// Items the system can truthfully use from this input.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recoverable: Vec<ImportTriageItem>,
    /// Items the system cannot currently recover or edit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_recoverable: Vec<ImportTriageItem>,
    /// Suggested next actions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next_steps: Vec<ImportTriageNextStep>,
}

impl ImportTriageReport {
    /// Return true when the source descriptor parsed and canonicalized.
    #[must_use]
    pub fn input_accepted(&self) -> bool {
        self.outcome != ImportTriageOutcome::InvalidInput
    }
}

/// Source class routed through import triage.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportTriageSourceKind {
    /// Descriptor produced by external clean-character analysis.
    ExternalCleanCharacterDescriptor,
}

/// Product-facing import triage outcome.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportTriageOutcome {
    /// Strict verification accepted an exact editable known-base program.
    ExactEditableRecovery,
    /// A known-base candidate exists, but exact editability is not proven.
    KnownBasePartialDiagnostic,
    /// The input is analyzable, but unsupported by current grammars.
    DiagnosticOnlyUnsupported,
    /// The descriptor is invalid or cannot be canonicalized.
    InvalidInput,
}

/// Triage item describing recovered, eligible, partial, or unsupported data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportTriageItem {
    /// Stable item code.
    pub code: String,
    /// Item status.
    pub status: ImportTriageItemStatus,
    /// Human-readable subject.
    pub subject: String,
    /// Deterministic evidence or limitation text.
    pub evidence: String,
}

/// Triage item status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportTriageItemStatus {
    /// Strict proof accepted.
    RecoveredExact,
    /// Evidence is sufficient for a strict gate attempt.
    EligibleForStrictGate,
    /// Evidence is useful but not exact recovery.
    DiagnosticOnly,
    /// Current system cannot recover or edit this subject.
    Unsupported,
}

/// Suggested next action for a triaged import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportTriageNextStep {
    /// Stable action code.
    pub action: ImportTriageAction,
    /// Whether the action is available now.
    pub available_now: bool,
    /// User-facing action label.
    pub label: String,
    /// Reason this action is or is not available.
    pub reason: String,
}

/// Stable triage action.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportTriageAction {
    /// Use the exact recovered known-base program.
    OpenExactRecoveredProgram,
    /// Review known-base diagnostic evidence.
    ReviewKnownBaseDiagnostics,
    /// Save or inspect the diagnostic report.
    SaveDiagnosticReport,
}

/// Triage an external clean-character descriptor.
#[must_use]
pub fn triage_external_clean_character_import(
    input: &ExternalCleanCharacterMeshInput,
    config: &ExternalCharacterAnalysisConfig,
) -> ImportTriageReport {
    let analysis = analyze_external_clean_character_mesh(input, config);
    let strict_recovery = strict_recovery_for_exact_character_candidate(input, &analysis);
    let strict_recovery_proven = strict_recovery
        .as_ref()
        .is_some_and(|recovery| recovery.strict_success);
    let outcome = classify_triage_outcome(&analysis, strict_recovery_proven);
    let classification_confidence = if outcome == ImportTriageOutcome::InvalidInput {
        Confidence(0.0)
    } else {
        analysis
            .candidates
            .first()
            .map(|candidate| candidate.confidence)
            .unwrap_or(analysis.canonicalization.confidence)
    };
    let (recoverable, not_recoverable) = triage_items(&analysis, strict_recovery.as_ref(), outcome);
    let next_steps = triage_next_steps(outcome, strict_recovery.as_ref());

    ImportTriageReport {
        schema_version: IMPORT_TRIAGE_SCHEMA_VERSION,
        source_id: input.source_id.clone(),
        source_kind: ImportTriageSourceKind::ExternalCleanCharacterDescriptor,
        outcome,
        user_facing_label: user_facing_label(outcome).to_owned(),
        strict_recovery_proven,
        strict_recovery_confidence: Confidence(if strict_recovery_proven { 1.0 } else { 0.0 }),
        classification_confidence,
        external_character_analysis: analysis,
        strict_known_base_recovery: strict_recovery,
        recoverable,
        not_recoverable,
        next_steps,
    }
}

/// Build an invalid-input triage report for descriptors that cannot be parsed
/// into the external clean-character schema.
#[must_use]
pub fn invalid_external_character_import_triage_report(
    source_id: impl Into<String>,
    message: impl Into<String>,
) -> ImportTriageReport {
    let source_id = source_id.into();
    let message = message.into();
    let analysis = ExternalCharacterAnalysisReport {
        schema_version: EXTERNAL_CHARACTER_ANALYSIS_SCHEMA_VERSION,
        source_id: source_id.clone(),
        canonicalization: ExternalCharacterCanonicalization {
            accepted: false,
            canonical_units: "meters".to_owned(),
            canonical_coordinate_system: "y_up_z_forward_right_handed".to_owned(),
            scale_to_meters: 1.0,
            axis_remap_required: false,
            scale_normalization_required: false,
            handedness_exact: false,
            canonical_bounds_min: [0.0; 3],
            canonical_bounds_max: [0.0; 3],
            confidence: Confidence(0.0),
        },
        candidates: Vec::new(),
        best_candidate_id: None,
        outcome: ExternalCharacterImportOutcome::InvalidInput,
        rejection_reasons: vec![ExternalCharacterRejectionReason {
            path: "descriptor".to_owned(),
            message: message.clone(),
        }],
        issues: vec![ExternalCharacterIssue {
            severity: ExternalCharacterIssueSeverity::Error,
            path: "descriptor".to_owned(),
            message,
        }],
    };
    let (recoverable, not_recoverable) =
        triage_items(&analysis, None, ImportTriageOutcome::InvalidInput);

    ImportTriageReport {
        schema_version: IMPORT_TRIAGE_SCHEMA_VERSION,
        source_id,
        source_kind: ImportTriageSourceKind::ExternalCleanCharacterDescriptor,
        outcome: ImportTriageOutcome::InvalidInput,
        user_facing_label: user_facing_label(ImportTriageOutcome::InvalidInput).to_owned(),
        strict_recovery_proven: false,
        strict_recovery_confidence: Confidence(0.0),
        classification_confidence: Confidence(0.0),
        external_character_analysis: analysis,
        strict_known_base_recovery: None,
        recoverable,
        not_recoverable,
        next_steps: triage_next_steps(ImportTriageOutcome::InvalidInput, None),
    }
}

fn strict_recovery_for_exact_character_candidate(
    input: &ExternalCleanCharacterMeshInput,
    analysis: &ExternalCharacterAnalysisReport,
) -> Option<KnownBaseCharacterRecoveryCaseReport> {
    if analysis.outcome != ExternalCharacterImportOutcome::ExactKnownBaseEligible {
        return None;
    }
    let artifact = canonical_character_artifact(input, analysis)?;
    Some(recover_known_base_character_mesh_artifact(
        format!("{}.import_triage", input.source_id),
        artifact,
    ))
}

fn canonical_character_artifact(
    input: &ExternalCleanCharacterMeshInput,
    analysis: &ExternalCharacterAnalysisReport,
) -> Option<CharacterMeshArtifact> {
    let mut artifact = CharacterMeshArtifact {
        id: format!("{}.canonical_character_descriptor", input.source_id),
        canonical_units: analysis.canonicalization.canonical_units.clone(),
        coordinate_system: analysis
            .canonicalization
            .canonical_coordinate_system
            .clone(),
        semantic_descriptor_fingerprint: input.semantic_descriptor_fingerprint.clone()?,
        raw_geometry_size: input.raw_geometry_size,
        connected_component_count: input.connected_component_count,
        bounds_min: analysis.canonicalization.canonical_bounds_min,
        bounds_max: analysis.canonicalization.canonical_bounds_max,
        topology_fingerprint: input.topology_fingerprint.clone()?,
        canonical_position_fingerprint: input.canonical_position_fingerprint.clone()?,
        artifact_fingerprint: String::new(),
    };
    artifact.artifact_fingerprint = character_mesh_artifact_fingerprint(&artifact);
    Some(artifact)
}

fn classify_triage_outcome(
    analysis: &ExternalCharacterAnalysisReport,
    strict_recovery_proven: bool,
) -> ImportTriageOutcome {
    if analysis.outcome == ExternalCharacterImportOutcome::InvalidInput {
        return ImportTriageOutcome::InvalidInput;
    }
    if strict_recovery_proven {
        return ImportTriageOutcome::ExactEditableRecovery;
    }
    if !analysis.candidates.is_empty() {
        return ImportTriageOutcome::KnownBasePartialDiagnostic;
    }
    ImportTriageOutcome::DiagnosticOnlyUnsupported
}

fn triage_items(
    analysis: &ExternalCharacterAnalysisReport,
    strict_recovery: Option<&KnownBaseCharacterRecoveryCaseReport>,
    outcome: ImportTriageOutcome,
) -> (Vec<ImportTriageItem>, Vec<ImportTriageItem>) {
    let mut recoverable = Vec::new();
    let mut not_recoverable = Vec::new();
    if analysis.canonicalization.accepted {
        recoverable.push(item(
            "canonical_descriptor",
            ImportTriageItemStatus::DiagnosticOnly,
            "Canonical clean-character descriptor",
            "units, axes, handedness, and bounds were normalized for analysis",
        ));
    } else {
        not_recoverable.push(item(
            "canonical_descriptor",
            ImportTriageItemStatus::Unsupported,
            "Canonical clean-character descriptor",
            "input units, axes, schema, or bounds prevent canonicalization",
        ));
    }

    match outcome {
        ImportTriageOutcome::ExactEditableRecovery => {
            let recovery = strict_recovery.expect("exact outcome requires strict recovery");
            recoverable.push(item(
                "exact_known_base_program",
                ImportTriageItemStatus::RecoveredExact,
                "Exact known-base character program",
                format!(
                    "strict verification accepted with {} matched base and compression {:.3}",
                    recovery.matched_base_count, recovery.program_compression
                ),
            ));
        }
        ImportTriageOutcome::KnownBasePartialDiagnostic => {
            if let Some(candidate) = analysis.candidates.first() {
                recoverable.push(item(
                    "known_base_candidate",
                    ImportTriageItemStatus::DiagnosticOnly,
                    "Known-base character candidate",
                    format!(
                        "best candidate {} scored {:.3}, but strict recovery was not proven",
                        candidate.id, candidate.confidence.0
                    ),
                ));
            }
            not_recoverable.push(item(
                "exact_editable_recovery",
                ImportTriageItemStatus::Unsupported,
                "Exact editable recovery",
                "strict verification did not accept an exact recovered program",
            ));
        }
        ImportTriageOutcome::DiagnosticOnlyUnsupported => {
            not_recoverable.push(item(
                "known_base_character_program",
                ImportTriageItemStatus::Unsupported,
                "Known-base character program",
                "no known-base candidate reached the reporting confidence threshold",
            ));
            not_recoverable.push(item(
                "visual_foundry_template",
                ImportTriageItemStatus::Unsupported,
                "Matching Visual Foundry template",
                "no current Visual Foundry family matches this external character descriptor",
            ));
        }
        ImportTriageOutcome::InvalidInput => {
            not_recoverable.push(item(
                "import_analysis",
                ImportTriageItemStatus::Unsupported,
                "Import analysis",
                "input descriptor failed validation or canonicalization",
            ));
        }
    }

    (recoverable, not_recoverable)
}

fn triage_next_steps(
    outcome: ImportTriageOutcome,
    strict_recovery: Option<&KnownBaseCharacterRecoveryCaseReport>,
) -> Vec<ImportTriageNextStep> {
    match outcome {
        ImportTriageOutcome::ExactEditableRecovery => {
            let case_id = strict_recovery
                .map(|recovery| recovery.case_id.as_str())
                .unwrap_or("unknown");
            vec![
                next_step(
                    ImportTriageAction::OpenExactRecoveredProgram,
                    true,
                    "Open exact recovered known-base program",
                    format!("strict known-base proof accepted for {case_id}"),
                ),
                next_step(
                    ImportTriageAction::SaveDiagnosticReport,
                    true,
                    "Save diagnostic report",
                    "triage report records the proof and boundary evidence",
                ),
            ]
        }
        ImportTriageOutcome::KnownBasePartialDiagnostic => vec![
            next_step(
                ImportTriageAction::ReviewKnownBaseDiagnostics,
                true,
                "Review known-base diagnostics",
                "candidate evidence exists, but exact recovery was not proven",
            ),
            next_step(
                ImportTriageAction::OpenExactRecoveredProgram,
                false,
                "Open exact recovered program",
                "strict verification has not accepted this input",
            ),
            next_step(
                ImportTriageAction::SaveDiagnosticReport,
                true,
                "Save diagnostic report",
                "partial evidence can guide prepared-template or grammar work",
            ),
        ],
        ImportTriageOutcome::DiagnosticOnlyUnsupported => vec![next_step(
            ImportTriageAction::SaveDiagnosticReport,
            true,
            "Save diagnostic report",
            "current grammars do not support exact recovery for this input",
        )],
        ImportTriageOutcome::InvalidInput => vec![next_step(
            ImportTriageAction::SaveDiagnosticReport,
            true,
            "Save validation report",
            "fix descriptor schema, units, axes, bounds, clean flags, or correspondences",
        )],
    }
}

fn user_facing_label(outcome: ImportTriageOutcome) -> &'static str {
    match outcome {
        ImportTriageOutcome::ExactEditableRecovery => "Recover exact editable program",
        ImportTriageOutcome::KnownBasePartialDiagnostic => "Known-base partial diagnostic",
        ImportTriageOutcome::DiagnosticOnlyUnsupported => "Diagnostic-only unsupported mesh",
        ImportTriageOutcome::InvalidInput => "Analyze import failed",
    }
}

fn item(
    code: &str,
    status: ImportTriageItemStatus,
    subject: &str,
    evidence: impl Into<String>,
) -> ImportTriageItem {
    ImportTriageItem {
        code: code.to_owned(),
        status,
        subject: subject.to_owned(),
        evidence: evidence.into(),
    }
}

fn next_step(
    action: ImportTriageAction,
    available_now: bool,
    label: &str,
    reason: impl Into<String>,
) -> ImportTriageNextStep {
    ImportTriageNextStep {
        action,
        available_now,
        label: label.to_owned(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use shape_character::corpus::{
        CharacterMeshArtifact, CharacterRawGeometrySize, KnownBaseCharacterMeshFeatures,
        generated_character_corpus, known_base_character_descriptor_for_features,
        known_base_character_signature_for_features,
    };

    use super::*;
    use crate::{
        analysis::Axis3,
        external_character::{
            CoordinateHandedness, ExternalCharacterCoordinateSystem,
            ExternalCharacterCorrespondenceObservation, external_character_artifact_fingerprint,
            external_input_from_character_mesh_artifact,
        },
    };

    #[test]
    fn exact_external_descriptor_runs_strict_recovery_before_claiming_editability() {
        let corpus = generated_character_corpus(2401);
        let input = external_input_from_character_mesh_artifact(
            "triage.character.exact",
            &corpus.cases[4].mesh,
        );

        let report = triage_external_clean_character_import(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(report.outcome, ImportTriageOutcome::ExactEditableRecovery);
        assert_eq!(report.user_facing_label, "Recover exact editable program");
        assert!(report.strict_recovery_proven);
        assert_eq!(report.strict_recovery_confidence, Confidence(1.0));
        assert_eq!(
            report.external_character_analysis.outcome,
            ExternalCharacterImportOutcome::ExactKnownBaseEligible
        );
        assert!(
            report
                .strict_known_base_recovery
                .as_ref()
                .is_some_and(|recovery| recovery.strict_success)
        );
        assert!(report.recoverable.iter().any(|item| {
            item.code == "exact_known_base_program"
                && item.status == ImportTriageItemStatus::RecoveredExact
        }));
    }

    #[test]
    fn partial_known_base_diagnostic_never_gets_exact_editable_label() {
        let corpus = generated_character_corpus(2402);
        let mut input = external_input_from_character_mesh_artifact(
            "triage.character.partial",
            &corpus.cases[2].mesh,
        );
        input.topology_fingerprint = None;
        input.canonical_position_fingerprint = None;
        input.semantic_descriptor_fingerprint = None;

        let report = triage_external_clean_character_import(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ImportTriageOutcome::KnownBasePartialDiagnostic
        );
        assert_eq!(report.user_facing_label, "Known-base partial diagnostic");
        assert!(!report.strict_recovery_proven);
        assert_eq!(report.strict_recovery_confidence, Confidence(0.0));
        assert!(report.strict_known_base_recovery.is_none());
        assert!(report.not_recoverable.iter().any(|item| {
            item.code == "exact_editable_recovery"
                && item.status == ImportTriageItemStatus::Unsupported
        }));
    }

    #[test]
    fn invalid_descriptor_writes_invalid_input_triage() {
        let corpus = generated_character_corpus(2403);
        let mut input = external_input_from_character_mesh_artifact(
            "triage.character.invalid",
            &corpus.cases[0].mesh,
        );
        input.correspondences = vec![
            ExternalCharacterCorrespondenceObservation {
                id: "obs.duplicate".to_owned(),
                target_id: "character.body".to_owned(),
                observed_count: 1,
                max_error_meters: 0.0,
            },
            ExternalCharacterCorrespondenceObservation {
                id: "obs.duplicate".to_owned(),
                target_id: "head.cranium".to_owned(),
                observed_count: 1,
                max_error_meters: 0.0,
            },
        ];

        let report = triage_external_clean_character_import(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(report.outcome, ImportTriageOutcome::InvalidInput);
        assert!(!report.input_accepted());
        assert!(!report.strict_recovery_proven);
        assert_eq!(report.classification_confidence, Confidence(0.0));
        assert!(report.external_character_analysis.canonicalization.accepted);
        assert!(report.external_character_analysis.candidates.is_empty());
        assert!(
            report
                .not_recoverable
                .iter()
                .any(|item| item.code == "import_analysis")
        );
    }

    #[test]
    fn parse_failure_report_is_invalid_input_without_stale_proof() {
        let report = invalid_external_character_import_triage_report(
            "triage.character.hidden_field",
            "unknown field `source_program`",
        );

        assert_eq!(report.outcome, ImportTriageOutcome::InvalidInput);
        assert_eq!(report.user_facing_label, "Analyze import failed");
        assert!(!report.strict_recovery_proven);
        assert!(report.strict_known_base_recovery.is_none());
        assert_eq!(report.classification_confidence, Confidence(0.0));
        assert_eq!(
            report.external_character_analysis.outcome,
            ExternalCharacterImportOutcome::InvalidInput
        );
        assert!(
            report
                .external_character_analysis
                .issues
                .iter()
                .any(|issue| {
                    issue.path == "descriptor" && issue.message.contains("source_program")
                })
        );
    }

    #[test]
    fn unsupported_descriptor_stays_diagnostic_only_without_foundry_claim() {
        let features = KnownBaseCharacterMeshFeatures {
            symmetric: true,
            asymmetric: false,
            posed: false,
            clothed: false,
            hair: false,
            topology_edited: true,
        };
        let mesh = mesh_for_features("triage.character.unsupported", features);
        let input =
            external_input_from_character_mesh_artifact("triage.character.unsupported", &mesh);

        let report = triage_external_clean_character_import(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ImportTriageOutcome::DiagnosticOnlyUnsupported
        );
        assert!(!report.strict_recovery_proven);
        assert!(report.strict_known_base_recovery.is_none());
        assert!(report.not_recoverable.iter().any(|item| {
            item.code == "visual_foundry_template"
                && item.evidence.contains("no current Visual Foundry family")
        }));
    }

    #[test]
    fn centimeter_z_up_descriptor_can_still_prove_exact_after_canonicalization() {
        let corpus = generated_character_corpus(2404);
        let mut input = external_input_from_character_mesh_artifact(
            "triage.character.cm_z_up",
            &corpus.cases[0].mesh,
        );
        input.coordinate_system = ExternalCharacterCoordinateSystem {
            unit: "centimeters".to_owned(),
            right_axis: Axis3::PositiveX,
            up_axis: Axis3::PositiveZ,
            forward_axis: Axis3::NegativeY,
            handedness: CoordinateHandedness::RightHanded,
        };
        let original_min = input.bounds_min;
        let original_max = input.bounds_max;
        input.bounds_min = [
            original_min[0] * 100.0,
            -original_max[2] * 100.0,
            original_min[1] * 100.0,
        ];
        input.bounds_max = [
            original_max[0] * 100.0,
            -original_min[2] * 100.0,
            original_max[1] * 100.0,
        ];

        let report = triage_external_clean_character_import(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(report.outcome, ImportTriageOutcome::ExactEditableRecovery);
        assert!(
            report
                .external_character_analysis
                .canonicalization
                .axis_remap_required
        );
        assert!(
            report
                .external_character_analysis
                .canonicalization
                .scale_normalization_required
        );
        assert!(report.strict_recovery_proven);
    }

    fn mesh_for_features(
        id: &str,
        features: KnownBaseCharacterMeshFeatures,
    ) -> CharacterMeshArtifact {
        let signature = known_base_character_signature_for_features(features);
        let descriptor = known_base_character_descriptor_for_features(features);
        let mut mesh = CharacterMeshArtifact {
            id: id.to_owned(),
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
            bounds_min: bits_to_bounds(signature.bounds_min_bits),
            bounds_max: bits_to_bounds(signature.bounds_max_bits),
            topology_fingerprint: descriptor.topology_fingerprint,
            canonical_position_fingerprint: descriptor.canonical_position_fingerprint,
            artifact_fingerprint: String::new(),
        };
        mesh.artifact_fingerprint = external_character_artifact_fingerprint(&mesh);
        mesh
    }

    fn bits_to_bounds(bits: [u32; 3]) -> [f32; 3] {
        [
            f32::from_bits(bits[0]),
            f32::from_bits(bits[1]),
            f32::from_bits(bits[2]),
        ]
    }
}
